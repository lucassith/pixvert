use actix_web::{HttpResponse, web};
use crate::AppState;

pub async fn health(data: web::Data<AppState<'_>>) -> HttpResponse {
    return match data.cache.lock() {
        Ok(_) => HttpResponse::Ok().body("ok"),
        Err(e) => HttpResponse::InternalServerError().body(format!("{:#?}", e)),
    }
}
