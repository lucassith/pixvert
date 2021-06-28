use std::{borrow::Borrow, sync::{Arc, Mutex}};

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, web};
use fetcher::{Fetchable, FetcherProvider, http_fetcher::HttpFetcher};

mod fetcher;

struct AppState {
    fetcher_provider: Mutex<FetcherProvider>,
}

async fn index(req: HttpRequest, data: web::Data<AppState>) -> HttpResponse {
    let fetcher_provider = data.fetcher_provider.lock().unwrap();

    let found_url = &req.match_info().get("tail").unwrap().to_string();
    println!("{:#?}", found_url);

    let fetcher = fetcher_provider.get(found_url);

    match fetcher {
        Option::Some(fetcher) => {
            return HttpResponse::Accepted().finish();
        }
        Option::None => {
            return HttpResponse::BadRequest().finish();
        }
    }
    
    
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let fetcher_provider = web::Data::new(AppState {
        fetcher_provider: Mutex::new(FetcherProvider::new(
            Vec::from([
                Arc::new(Box::new(HttpFetcher::new()) as Box<dyn Fetchable + Sync + Send>)
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
