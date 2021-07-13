use std::{sync::{Arc, Mutex}};

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};
use fetcher::{Fetchable, FetcherProvider, http_fetcher::HttpFetcher};
use actix_web::dev::BodyEncoding;
use actix_web::http::ContentEncoding;

mod image;
mod fetcher;
mod cache;

struct AppState {
    fetcher_provider: Mutex<FetcherProvider>,
}

async fn index(req: HttpRequest, data: web::Data<AppState>) -> HttpResponse {
    let found_url = &req.match_info().get("tail").unwrap().to_string();
    log::info!("Found url: {:#}", found_url);

    let fetcher_provider = data.fetcher_provider.lock().unwrap();
    let fetcher = fetcher_provider.get(found_url);
    if fetcher.is_none() {
        return HttpResponse::NotFound().finish();
    }
    let fetched_object = fetcher.unwrap().fetch(found_url).await;

    return match fetched_object {
        Ok(object) => {
            HttpResponse::Ok()
                .content_type(object.mime.to_string())
                .body(object.bytes)
        }
        Err(e) => {
            HttpResponse::BadRequest().finish()
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    log4rs::init_file("logger-config.yml", Default::default()).unwrap();
    let cache: Arc<Mutex<dyn cache::Cachable<fetcher::FetchedObject> + Send + Sync>> = Arc::new(Mutex::new(cache::memory_cache::MemoryCache::new()));
    let fetcher_provider = web::Data::new(AppState {
        fetcher_provider: Mutex::new(FetcherProvider::new(
            Vec::from([
                Arc::new(Box::new(HttpFetcher::new(
                    cache.clone()
                )) as Box<dyn Fetchable + Sync + Send>)
            ])
        ))
    });

    HttpServer::new(move || {
        App::new()
            .app_data(fetcher_provider.clone())
            .route("/{tail:.*}", web::get().to(index))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
