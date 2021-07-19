use std::{sync::{Arc, Mutex}};

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};

use fetcher::{http_fetcher::HttpFetcher};

use crate::fetcher::{FetchError, FetchableService, FetchedObject};
use crate::service_provider::ServiceProvider;
use crate::image::encoder::ImageEncoderService;
use crate::image::decoder::ImageDecoderService;
use crate::image::decoder::image_png_jpg_decoder::ImagePngJpgDecoder;
use crate::image::encoder::image_webp_encoder::ImageWebpEncoder;
use crate::image::{EncodedImage, DecodedImage};
use std::result::Result::Err;
use crate::image::scaler::ImageScalerService;
use crate::image::scaler::lanczos3_scaler::Lanczos3ImageScaler;


mod image;
mod fetcher;
mod cache;
mod service_provider;

struct AppState {
    fetcher_provider: Mutex<ServiceProvider<dyn FetchableService + Send + Sync>>,
    decoder_provider: Mutex<ServiceProvider<dyn ImageDecoderService + Send + Sync>>,
    encoder_provider: Mutex<ServiceProvider<dyn ImageEncoderService + Send + Sync>>,
    scaler_provider: Mutex<ServiceProvider<dyn ImageScalerService + Send + Sync>>,
}

async fn index(req: HttpRequest, data: web::Data<AppState>) -> HttpResponse {
    let found_url = &req.match_info().get("tail").unwrap().to_string();
    log::info!("Found url: {:#}", found_url);

    let fetcher_provider = data.fetcher_provider.lock().unwrap();
    let fetcher = fetcher_provider.get(found_url);
    if fetcher.is_none() {
        return HttpResponse::Gone().finish();
    }
    let fetched_object = fetcher.unwrap().fetch(found_url).await;
    if let Err(err) = &fetched_object {
        return match err {
            FetchError::ObjectNotFound => {
                HttpResponse::NotFound().finish()
            }
            FetchError::FetchFailed(e) => {
                HttpResponse::NotFound().body(e)
            }
        }
    }
    let fetched_object = fetched_object.unwrap();
    let decoder_provider = data.decoder_provider.lock().unwrap();
    let decoder = decoder_provider.get(
        &String::from(fetched_object.mime.to_string())
    );
    let mut decoded_object = decoder.unwrap().decode(found_url, fetched_object).await.unwrap();
    let width = req.match_info().get("width").unwrap_or("no-width");
    let height = req.match_info().get("height").unwrap_or("no-height");
    match (width.parse::<u32>(), height.parse::<u32>()) {
        (Ok(width), Ok(height)) => {
            let scaler_provider = data.scaler_provider.lock().unwrap().get(&String::from("")).unwrap();
            decoded_object = scaler_provider.scale(found_url, &decoded_object, (width, height)).await.unwrap();
        }
        (Err(we), Err(he))=> {
            log::trace!("Failed to parse width {} and height {}. Err: {} {}", width, height, we, he);
        }
        (_, Err(he))=> {
            log::error!("Failed to parse height {}. Err: {}", height, he);
        }
        (Err(we), _)=> {
            log::error!("Failed to parse width {}. Err: {}", width, we);
        }
    }
    let output_format: String = match req.match_info().get("format") {
        None => {
            decoded_object.from.to_string()
        }
        Some(format) => {
            match format {
                "webp" => String::from("image/webp"),
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
            let encoded_image = encoder.encode(found_url, decoded_object).await.unwrap();
            HttpResponse::Ok()
                .content_type(encoded_image.output_mime)
                .body(encoded_image.image)
        }
    }
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    log4rs::init_file("logger-config.yml", Default::default()).unwrap();
    let fetched_object_cache: Arc<Mutex<dyn cache::Cachable<FetchedObject> + Send + Sync>> = Arc::new(Mutex::new(cache::memory_cache::MemoryCache::new()));
    let encoded_image_cache: Arc<Mutex<dyn cache::Cachable<EncodedImage> + Send + Sync>> = Arc::new(Mutex::new(cache::memory_cache::MemoryCache::new()));
    let decoded_image_cache: Arc<Mutex<dyn cache::Cachable<DecodedImage> + Send + Sync>> = Arc::new(Mutex::new(cache::memory_cache::MemoryCache::new()));
    let scaled_image_cache: Arc<Mutex<dyn cache::Cachable<DecodedImage> + Send + Sync>> = Arc::new(Mutex::new(cache::memory_cache::MemoryCache::new()));
    let app_state = web::Data::new(AppState {
        fetcher_provider: Mutex::new(ServiceProvider::new(
            Vec::from([
                Arc::new(Box::new(HttpFetcher::new(
                    fetched_object_cache.clone()
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
                )) as Box<dyn ImageEncoderService + Sync + Send>)
            ])
        )),
        scaler_provider: Mutex::new(ServiceProvider::new(
            Vec::from([
                Arc::new(Box::new(Lanczos3ImageScaler::new(
                    scaled_image_cache.clone()
                )) as Box<dyn ImageScalerService + Sync + Send>)
            ])
        ))
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
        .await
}
