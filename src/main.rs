use std::{sync::{Arc, Mutex}};

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};

use fetcher::{http_fetcher::HttpFetcher};

use crate::fetcher::FetchableService;
use crate::service_provider::ServiceProvider;

mod image;
mod fetcher;
mod cache;
mod service_provider;

struct AppState {
    fetcher_provider: Mutex<ServiceProvider<dyn FetchableService + Send + Sync>>,
}

async fn index(req: HttpRequest, data: web::Data<AppState>) -> HttpResponse {
    let found_url = &req.match_info().get("tail").unwrap().to_string();
    log::info!("Found url: {:#}", found_url);

    let fetcher_provider = data.fetcher_provider.lock().unwrap();
    let fetcher = fetcher_provider.get(found_url);
    if fetcher.is_none() {
        return HttpResponse::NotFound().finish();
    }
    unimplemented!()
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    log4rs::init_file("logger-config.yml", Default::default()).unwrap();
    let cache: Arc<Mutex<dyn cache::Cachable<fetcher::FetchedObject> + Send + Sync>> = Arc::new(Mutex::new(cache::memory_cache::MemoryCache::new()));
    let app_state = web::Data::new(AppState {
        fetcher_provider: Mutex::new(ServiceProvider::new(
            Vec::from([
                Arc::new(Box::new(HttpFetcher::new(
                    cache.clone()
                )) as Box<dyn FetchableService + Sync + Send>)
            ])
        ))
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/{tail:.*}", web::get().to(index))
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
