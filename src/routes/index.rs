use std::mem::size_of_val;

use actix_web::{HttpRequest, HttpResponse, web};
use log::info;

use crate::AppState;
use crate::encoder::OutputFormat;
use crate::fetcher::FetchError;
use crate::fetcher::HTTP_ADDITIONAL_DATA_HEADERS_KEY;

pub async fn index(req: HttpRequest, data: web::Data<AppState<'_>>) -> HttpResponse {
    return generate_image(req, data, false).await;
}

pub async fn index_with_ratio(req: HttpRequest, data: web::Data<AppState<'_>>) -> HttpResponse {
    return generate_image(req, data, true).await;
}

impl From<FetchError> for HttpResponse {
    fn from(e: FetchError) -> Self {
        return match e {
            FetchError::NotFound => HttpResponse::NotFound().body(format!("{:#?}", e)),
            FetchError::NoAccess => HttpResponse::Forbidden().body(format!("{:#?}", e)),
            FetchError::InvalidFormat => HttpResponse::UnprocessableEntity().body(format!("{:#?}", e)),
            _ => HttpResponse::InternalServerError().body(format!("{:#?}", e)),
        }
    }
}

pub async fn generate_image(req: HttpRequest, data: web::Data<AppState<'_>>, keep_ratio: bool) -> HttpResponse {
    let resource_url = &req.match_info().get("tail").unwrap().to_string();
    let resource_uri = urlencoding::decode(resource_url).unwrap();
    let resource = match data
        .fetcher
        .lock()
        .unwrap()
        .fetch(&resource_uri.to_string())
        .await {
            Ok(r) => r,
            Err(e) => return e.into(),
        };



    info!("Received image in format: {} - size: {}", &resource.content_type, size_of_val(&*resource.content.as_slice()));

    let mut output_format = match req
        .match_info()
        .get("format")
        .unwrap_or(resource.content_type.as_str())
        .parse::<OutputFormat>() {
            Ok(f) => f,
            Err(_) => return HttpResponse::UnprocessableEntity().body(format!("Invalid format: {}", req.match_info().get("format").unwrap_or(resource.content_type.as_str()))),
        };


    info!("Image will be converted to: {}", output_format);


    let mut response = HttpResponse::Ok();

    if let Some(http_additional_data) = resource.additional_data.get(HTTP_ADDITIONAL_DATA_HEADERS_KEY) {
        for (header_name, header_value) in http_additional_data.into_iter() {
            response.insert_header((header_name.clone(), header_value.clone()));
        }
    }

    let resource_id = resource.id.clone();
    let mut img = match data.decoder.lock().unwrap().decode(&resource_id, resource) {
        Ok(img) => img,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().body(format!("{:#?}", err));
        }
    };

    let width = req.match_info().get("width").unwrap_or("no-width");
    let height = req.match_info().get("height").unwrap_or("no-height");

    match (width.parse::<usize>(), height.parse::<usize>()) {
        (Ok(width), Ok(height)) => {
            if keep_ratio {
                img = data.resizer.lock().unwrap().resize(&resource_id, img, (width, height)).unwrap();
            } else {
                img = data.resizer.lock().unwrap().resize_exact(&resource_id, img, (width, height)).unwrap();
            }
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

    let encoded_image = data.encoder.lock().unwrap().encode(
        &resource_id,
        img,
        output_format,
    ).unwrap();

    return response.content_type(encoded_image.content_type).body(encoded_image.image);
}
