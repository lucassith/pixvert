use std::{sync::{Arc, Mutex}};
use std::result::Result::Err;

use log::debug;

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};

use fetcher::http_fetcher::HttpFetcher;

use crate::fetcher::{FetchableService, FetchedObject, FetchError};
use crate::image::{DecodedImage, EncodedImage};
use crate::image::decoder::image_png_jpg_decoder::ImagePngJpgDecoder;
use crate::image::decoder::ImageDecoderService;
use crate::image::encoder::image_webp_encoder::ImageWebpEncoder;
use crate::image::encoder::ImageEncoderService;
use crate::image::scaler::ImageScalerService;
use crate::image::scaler::lanczos3_scaler::Lanczos3ImageScaler;
use crate::service_provider::ServiceProvider;
use crate::image::encoder::image_png_jpg_encoder::{ImagePngJpgEncoder, ImagePngJpgEncoderType};
use serde::Deserialize;
use crate::http_cache::request_cache_handler::RequestCacheHandler;

mod image;
mod fetcher;
mod cache;
mod service_provider;
mod http_cache;

static IMAGE_CACHE_HASH_LITERAL: &str = "Image-Cache-Hash";

struct AppState {
    fetcher_provider: Mutex<ServiceProvider<dyn FetchableService + Send + Sync>>,
    decoder_provider: Mutex<ServiceProvider<dyn ImageDecoderService + Send + Sync>>,
    encoder_provider: Mutex<ServiceProvider<dyn ImageEncoderService + Send + Sync>>,
    scaler_provider: Mutex<ServiceProvider<dyn ImageScalerService + Send + Sync>>,
}

#[derive(Debug, Deserialize)]
pub struct ImageConversionRequest {
    quality: Option<f32>,
}

async fn index(req: HttpRequest, data: web::Data<AppState>, info: web::Query<ImageConversionRequest>) -> HttpResponse {
    let resource_url = &req.match_info().get("tail").unwrap().to_string();
    let quality = info.quality.unwrap_or(95f32);
    if quality <= 0f32 || quality > 100f32 {
        return HttpResponse::BadRequest().body("quality must be between 0 and 100");
    }
    debug!("Received quality {}", quality);
    log::info!("Found url: {:#}", resource_url);

    let fetcher_provider = data.fetcher_provider.lock().unwrap();
    let fetcher = fetcher_provider.get(resource_url);
    if fetcher.is_none() {
        return HttpResponse::Gone().finish();
    }
    let fetched_object = fetcher.unwrap().fetch(resource_url).await;
    if let Err(err) = &fetched_object {
        return match err {
            FetchError::ObjectNotFound => {
                HttpResponse::NotFound().finish()
            }
            FetchError::FetchFailed(e) => {
                HttpResponse::NotFound().body(e)
            }
        };
    }
    let fetched_object = fetched_object.unwrap();
    let decoder_provider = data.decoder_provider.lock().unwrap();
    let decoder = decoder_provider.get(
        &String::from(fetched_object.mime.to_string())
    );
    let mut decoded_object = decoder.unwrap().decode(&req.path().to_string(), fetched_object).await.unwrap();
    let width = req.match_info().get("width").unwrap_or("no-width");
    let height = req.match_info().get("height").unwrap_or("no-height");

    match (width.parse::<u32>(), height.parse::<u32>()) {
        (Ok(width), Ok(height)) => {
            let scaler_provider = data.scaler_provider.lock().unwrap().get(&String::from("")).unwrap();
            decoded_object = scaler_provider.scale(&req.path().to_string(), &decoded_object, (width, height)).await.unwrap();
        }
        (Err(we), Err(he)) => {
            log::trace!("Failed to parse width {} and height {}. Err: {} {}", width, height, we, he);
        }
        (_, Err(he)) => {
            log::error!("Failed to parse height {}. Err: {}", height, he);
        }
        (Err(we), _) => {
            log::error!("Failed to parse width {}. Err: {}", width, we);
        }
    }
    let output_format: String = match req.match_info().get("format") {
        None => {
            decoded_object.from.to_string()
        }
        Some(format) => {
            match format.to_lowercase().as_str() {
                "webp" => String::from("image/webp"),
                "jpg" => mime::IMAGE_JPEG.to_string(),
                "png" => mime::IMAGE_PNG.to_string(),
                _ => String::from("unknown")
            }
        }
    };
    let encoder_provider = data.encoder_provider.lock().unwrap();
    let encoder = encoder_provider.get(
        &String::from(output_format)
    );
    return match encoder {
        None => {
            HttpResponse::NotAcceptable().finish()
        }
        Some(encoder) => {
            let encoded_image = encoder.encode(
                &req.path().to_string(),
                decoded_object,
                quality
            ).await.unwrap();
            HttpResponse::Ok()
                .content_type(encoded_image.output_mime)
                .body(encoded_image.image)
        }
    };
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    log4rs::init_file("logger-config.yml", Default::default()).unwrap();
    let fetched_object_cache: Arc<Mutex<dyn cache::Cachable<FetchedObject> + Send + Sync>> = Arc::new(Mutex::new(cache::file_cache::FileCache::new(&String::from("/tmp/pixvert_image_cache/fetched_object"))));
    let encoded_image_cache: Arc<Mutex<dyn cache::Cachable<EncodedImage> + Send + Sync>> = Arc::new(Mutex::new(cache::file_cache::FileCache::new(&String::from("/tmp/pixvert_image_cache/encoded_image"))));
    let decoded_image_cache: Arc<Mutex<dyn cache::Cachable<DecodedImage> + Send + Sync>> = Arc::new(Mutex::new(cache::file_cache::FileCache::new(&String::from("/tmp/pixvert_image_cache/decoded_image"))));
    let scaled_image_cache: Arc<Mutex<dyn cache::Cachable<DecodedImage> + Send + Sync>> = Arc::new(Mutex::new(cache::file_cache::FileCache::new(&String::from("/tmp/pixvert_image_cache/scaled_image"))));
    let app_state = web::Data::new(AppState {
        fetcher_provider: Mutex::new(ServiceProvider::new(
            Vec::from([
                Arc::new(Box::new(HttpFetcher::new(
                    fetched_object_cache.clone(),
                    Box::new(RequestCacheHandler{}),
                )) as Box<dyn FetchableService + Sync + Send>)
            ])
        )),
        decoder_provider: Mutex::new(ServiceProvider::new(
            Vec::from([
                Arc::new(Box::new(ImagePngJpgDecoder::new(
                    decoded_image_cache.clone()
                )) as Box<dyn ImageDecoderService + Sync + Send>)
            ])
        )),
        encoder_provider: Mutex::new(ServiceProvider::new(
            Vec::from([
                Arc::new(Box::new(ImageWebpEncoder::new(
                    encoded_image_cache.clone()
                )) as Box<dyn ImageEncoderService + Sync + Send>),
                Arc::new(Box::new(ImagePngJpgEncoder::new(
                    encoded_image_cache.clone(),
                    ImagePngJpgEncoderType::JPG
                )) as Box<dyn ImageEncoderService + Sync + Send>),
                Arc::new(Box::new(ImagePngJpgEncoder::new(
                    encoded_image_cache.clone(),
                    ImagePngJpgEncoderType::PNG
                )) as Box<dyn ImageEncoderService + Sync + Send>)
            ])
        )),
        scaler_provider: Mutex::new(ServiceProvider::new(
            Vec::from([
                Arc::new(Box::new(Lanczos3ImageScaler::new(
                    scaled_image_cache.clone()
                )) as Box<dyn ImageScalerService + Sync + Send>)
            ])
        )),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/{width}_{height}/{format}/{tail:.*}", web::get().to(index))
            .route("/{width}_{height}/{tail:.*}", web::get().to(index))
            .route("/{format}/{tail:.*}", web::get().to(index))
            .route("/{tail:.*}", web::get().to(index))
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await;
    std::fs::remove_dir_all("/tmp/pixvert_image_cache").unwrap_or_default();
    Result::Ok(())
}
