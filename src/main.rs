use std::fs::OpenOptions;
use std::io::{LineWriter, Write};
use std::sync::{Arc, Mutex};

use actix_web::{App, HttpServer, web};
use figment::Figment;
use figment::providers::{Format, Yaml};
use log::error;

use crate::cache::{CacheEngine, HashMapCacheEngine};
use crate::config::Config;
use crate::decoder::{CachedImageDecoder, ImageDecoder};
use crate::encoder::{AllInOneCachedImageEncoder, ImageEncoder};
use crate::fetcher::{Fetcher, ReqwestImageFetcher, Resource};
use crate::resizer::{CachedResizer, Resizer};
use crate::routes::health::health;
use crate::routes::index::{index, index_with_ratio};

mod image;
mod cache;
mod fetcher;
mod tagged_element;
mod config;
mod routes;
mod resizer;
mod encoder;
mod decoder;

pub struct AppState<'a> {
    config: Mutex<Config>,
    fetcher: Mutex<Box<dyn Fetcher<Resource> + Send>>,
    decoder: Mutex<Box<dyn ImageDecoder + Send>>,
    resizer: Mutex<Box<dyn Resizer + Send>>,
    encoder: Mutex<Box<dyn ImageEncoder + Send>>,
    cache: &'a Mutex<Box<dyn CacheEngine + Send>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    log4rs::init_file("logger-config.yml", Default::default()).unwrap();
    let config: Config = match Figment::new()
        .merge(Yaml::file("app.yml"))
        .extract() {
        Ok(c) => c,
        Err(err) => {
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
    };
    let cache_engine = HashMapCacheEngine::default();
    let mutex_cache_engine = Mutex::from(Box::from(cache_engine) as Box<dyn CacheEngine + Send>);
    let cache = mutex_cache_engine;
    let arc_cache = &*Box::leak(Box::new(Arc::new(cache)));


    HttpServer::new(move || {
        let fetcher = ReqwestImageFetcher{
            cache: &*arc_cache,
            config: config.clone(),
        };
        let resizer = CachedResizer{
            cache: &*arc_cache,
        };
        let encoder = AllInOneCachedImageEncoder{cache: &*arc_cache};
        let decoder = CachedImageDecoder{cache: &*arc_cache};

        let app_state = web::Data::new(AppState {
            config: Mutex::new(config.clone()),
            fetcher: Mutex::new(Box::new(fetcher)),
            resizer: Mutex::new(Box::new(resizer)),
            encoder: Mutex::new(Box::new(encoder)),
            decoder: Mutex::new(Box::new(decoder)),
            cache: &*arc_cache
        });
        App::new()
            .app_data(app_state.clone().clone())
            .route("/_health", web::get().to(health))
            .route("/cache", web::get().to(health))
            .route("/{width}_{height}/keep-ratio/{format}/{tail:.*}", web::get().to(index_with_ratio))
            .route("/{width}_{height}/keep-ratio/{tail:.*}", web::get().to(index_with_ratio))
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
