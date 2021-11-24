use actix_web::{HttpResponse, web};

use crate::AppState;

pub async fn health(data: web::Data<AppState>) -> HttpResponse {
    if let Err(e) = data.cache.write() {
        return HttpResponse::InternalServerError().body(format!("{:#?}", e));
    }
    if let Err(e) = data.decoder.lock() {
        return HttpResponse::InternalServerError().body(format!("{:#?}", e));
    }
    if let Err(e) = data.encoder.lock() {
        return HttpResponse::InternalServerError().body(format!("{:#?}", e));
    }
    if let Err(e) = data.resizer.lock() {
        return HttpResponse::InternalServerError().body(format!("{:#?}", e));
    }
    if let Err(e) = data.config.lock() {
        return HttpResponse::InternalServerError().body(format!("{:#?}", e));
    }
    if let Err(e) = data.fetcher.lock() {
        return HttpResponse::InternalServerError().body(format!("{:#?}", e));
    }
    return HttpResponse::Ok().body("ok");
}
