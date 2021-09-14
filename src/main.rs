use std::{sync::{Arc, Mutex}};
use std::result::Result::Err;

use log::{debug, error};

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web, Responder};

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
use crate::config::Config;
use figment::Figment;
use figment::providers::{Yaml, Format};
use std::string::ParseError;
use std::fs::OpenOptions;
use std::io::{Write, LineWriter};
use actix_web::http::Uri;
use actix_web::http::uri::InvalidUri;

mod image;
mod fetcher;
mod cache;
mod service_provider;
mod http_cache;
mod config;

static IMAGE_CACHE_HASH_LITERAL: &str = "Image-Cache-Hash";

struct AppState {
    fetcher_provider: Mutex<ServiceProvider<dyn FetchableService + Send + Sync>>,
    decoder_provider: Mutex<ServiceProvider<dyn ImageDecoderService + Send + Sync>>,
    encoder_provider: Mutex<ServiceProvider<dyn ImageEncoderService + Send + Sync>>,
    scaler_provider: Mutex<ServiceProvider<dyn ImageScalerService + Send + Sync>>,
    config: Mutex<Config>,
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

    let resource_uri = match urlencoding::decode(resource_url).unwrap().parse::<Uri>() {
        Ok(uri) => {
            if let Some(host) = uri.host() {
                let found_allowed_domain = (&data.config.lock().unwrap().allow_from)
                    .clone()
                    .into_iter()
                    .find(|v| -> bool {
                        if let Some(uri_host) = &uri.host() {
                            return uri_host.ends_with(v);
                        }
                        return false;
                });
                if found_allowed_domain.is_some() {
                    uri
                } else {
                    return HttpResponse::Forbidden().body("Provided resource url Host is not set as allowed domain.");
                }
            } else {
                return HttpResponse::BadRequest().body("Resource URI doesn't include host.");
            }
        },
        Err(e) => {
            error!("Invalid resource URI. {:#?}", e);
            return HttpResponse::BadRequest().body(format!("Failed to parse resource URI: {}", e));
        }
    };

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
    drop(fetcher_provider);

    let fetched_object = fetched_object.unwrap();
    let decoder_provider = data.decoder_provider.lock().unwrap();
    let decoder = decoder_provider.get(
        &String::from(fetched_object.mime.to_string())
    );
    drop(decoder_provider);
    let mut decoded_object = decoder.unwrap().decode(&req.path().to_string(), &fetched_object).await.unwrap();
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
            let mut resp = HttpResponse::Ok();
            resp.content_type(encoded_image.output_mime);
            if let Some(overridden_cache) = (&data.config.lock().unwrap().overridden_cache).into_iter().find(|v| -> bool {
                if let Some(resource_host) = resource_uri.host() {
                    return resource_host.ends_with(&v.domain);
                }
                return false;
            }) {
                resp.append_header((actix_web::http::header::CACHE_CONTROL.to_string(), overridden_cache.cache_control.clone()));
            }
            else if let Some(cache_header) = fetched_object.cache_info.get(&actix_web::http::header::CACHE_CONTROL.to_string()) {
                resp.append_header((actix_web::http::header::CACHE_CONTROL.to_string(), cache_header.clone()));
            }
            return resp.body(encoded_image.image)
        }
    };
}
async fn health(data: web::Data<AppState>) -> HttpResponse {
    let services: Vec<bool> = vec!(
        data.config.is_poisoned(),
        data.decoder_provider.is_poisoned(),
        data.fetcher_provider.is_poisoned(),
        data.scaler_provider.is_poisoned(),
        data.encoder_provider.is_poisoned()
    );
    if services.into_iter().find(|&v| -> bool { v }).is_some() {
        return HttpResponse::ServiceUnavailable()
            .append_header((actix_web::http::header::CACHE_CONTROL.as_str(), "no-store"))
            .finish();
    }
    return HttpResponse::Ok()
        .append_header((actix_web::http::header::CACHE_CONTROL.as_str(), "no-store"))
        .finish();
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    log4rs::init_file("logger-config.yml", Default::default()).unwrap();
    let fetched_object_cache: Arc<Mutex<dyn cache::Cachable<FetchedObject> + Send + Sync>> = Arc::new(Mutex::new(cache::file_cache::FileCache::new(&String::from("/tmp/pixvert_image_cache/fetched_object"))));
    let encoded_image_cache: Arc<Mutex<dyn cache::Cachable<EncodedImage> + Send + Sync>> = Arc::new(Mutex::new(cache::file_cache::FileCache::new(&String::from("/tmp/pixvert_image_cache/encoded_image"))));
    let decoded_image_cache: Arc<Mutex<dyn cache::Cachable<DecodedImage> + Send + Sync>> = Arc::new(Mutex::new(cache::file_cache::FileCache::new(&String::from("/tmp/pixvert_image_cache/decoded_image"))));
    let scaled_image_cache: Arc<Mutex<dyn cache::Cachable<DecodedImage> + Send + Sync>> = Arc::new(Mutex::new(cache::file_cache::FileCache::new(&String::from("/tmp/pixvert_image_cache/scaled_image"))));
    let config: Result<Config, figment::error::Error> = Figment::new()
        .merge(Yaml::file("app.yml"))
        .extract();
    if let Err(err) = config {
        error!("{:#?}", err);
        let file = OpenOptions::new().create(true).write(true).read(true).open(
            "app.yml"
        ).unwrap();
        let mut file = LineWriter::new(file);
        file.write_all(
            &*serde_yaml::to_vec(&Config::default()).unwrap()
        ).unwrap();
        error!("Config 'app.yml' not found. Created new default config file.");
        return Result::Ok(());
    }
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
        config: Mutex::new(config.unwrap())
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/_health", web::get().to(health))
            .route("/{width}_{height}/{format}/{tail:.*}", web::get().to(index))
            .route("/{width}_{height}/{tail:.*}", web::get().to(index))
            .route("/{format}/{tail:.*}", web::get().to(index))
            .route("/{tail:.*}", web::get().to(index))
    })
        .bind("0.0.0.0:8080")?
        .run()
        .await;
    std::fs::remove_dir_all("/tmp/pixvert_image_cache").unwrap_or_default();
    Result::Ok(())
}
