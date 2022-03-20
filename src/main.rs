use std::fs::OpenOptions;
use std::io::{LineWriter, Write};
use std::sync::{Arc, Mutex, RwLock};

use actix_web::{App, HttpServer, web};
use figment::Figment;
use figment::providers::{Format, Yaml};
use log::{error, info, warn};

use crate::cache::{CacheEngine, HashMapCacheEngine};
use crate::cache::file_cache::FileCache;
use crate::config::{CacheType, Config};
use crate::decoder::{CachedImageDecoder, ImageDecoder};
use crate::encoder::{AllInOneCachedImageEncoder, ImageEncoder};
use crate::fetcher::{Fetcher, HttpImageFetcher, Resource};
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

pub struct AppState {
    config: Mutex<Config>,
    fetcher: Mutex<Box<dyn Fetcher<Resource> + Send>>,
    decoder: Mutex<Box<dyn ImageDecoder + Send>>,
    resizer: Mutex<Box<dyn Resizer + Send>>,
    encoder: Mutex<Box<dyn ImageEncoder + Send>>,
    cache: Arc<RwLock<Box<dyn CacheEngine + Send + Sync>>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    log4rs::init_file("logger-config.yml", Default::default()).unwrap();
    let config: Config = match Figment::new()
        .merge(Yaml::file("app.yml"))
        .extract() {
        Ok(c) => c,
        Err(err) => {
            let file = match OpenOptions::new().create_new(true).write(true).read(true).open(
                "app.yml"
            ) {
                Result::Ok(file) => file,
                Result::Err(e) => {
                    error!("Failed to create new application config. Reason: {}", e);
                    return Result::Ok(())
                }
            };
            let mut file = LineWriter::new(file);
            file.write_all(
                &*serde_yaml::to_vec(&Config::default()).unwrap()
            ).unwrap();
            error!("Config 'app.yml' not found. Created new default config file.");
            return Result::Ok(());
        }
    };
    let cache_engine: Box<dyn CacheEngine + Send + Sync> = match &config.cache.cache_type {
        CacheType::InMemory => Box::from(HashMapCacheEngine::default()) as Box<dyn CacheEngine + Send + Sync>,
        CacheType::File(path) => Box::from(FileCache::new(path)) as Box<dyn CacheEngine + Send + Sync>,
    };
    let mutex_cache_engine = RwLock::from(cache_engine);
    let arc_cache = Arc::new(mutex_cache_engine);
    let config_clone = config.clone();

    HttpServer::new(move || {
        let c_arc_cache = arc_cache.clone();
        let fetcher = HttpImageFetcher {
            cache: c_arc_cache.clone(),
            config: config_clone.clone(),
        };
        let resizer = CachedResizer {
            cache: c_arc_cache.clone(),
            config: config_clone.clone(),
        };
        let encoder = AllInOneCachedImageEncoder { cache: c_arc_cache.clone() };
        let decoder = CachedImageDecoder { cache: c_arc_cache.clone() };

        let app_state = web::Data::new(AppState {
            config: Mutex::new(config_clone.clone()),
            fetcher: Mutex::new(Box::new(fetcher)),
            resizer: Mutex::new(Box::new(resizer)),
            encoder: Mutex::new(Box::new(encoder)),
            decoder: Mutex::new(Box::new(decoder)),
            cache: c_arc_cache.clone(),
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
    if let CacheType::File(path) = &config.cache.cache_type {
        if path.starts_with(&String::from(std::env::temp_dir().to_string_lossy())) {
            info!("Cleaning temp dir: {}", path);
            std::fs::remove_dir_all(path).unwrap_or_default();
        } else {
            warn!("Unable to delete file cache catalog. Temp is set to {} and it is outside of system's tmp dir: {}. Please remove the cache dir.", path, std::env::temp_dir().to_str().unwrap());
        }

    }
    Result::Ok(())
}
