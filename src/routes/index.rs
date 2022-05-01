use std::mem::size_of_val;

use actix_web::{HttpRequest, HttpResponse, HttpResponseBuilder, web};
use log::{debug, info};

use crate::AppState;
use crate::encoder::OutputFormat;
use crate::fetcher::FetchError;
use crate::output_dimensions::OutputDimensions;
use crate::resizer::ResizeError;

pub async fn index(req: HttpRequest, data: web::Data<AppState>) -> HttpResponse {
    generate_image(req, data, false)
}

pub async fn index_with_ratio(req: HttpRequest, data: web::Data<AppState>) -> HttpResponse {
    generate_image(req, data, true)
}

impl From<FetchError> for HttpResponse {
    fn from(e: FetchError) -> Self {
        return match e {
            FetchError::NotFound => HttpResponse::NotFound().body(format!("{:#?}", e)),
            FetchError::NoAccess => HttpResponse::Forbidden().body(format!("{:#?}", e)),
            FetchError::InvalidFormat => HttpResponse::UnprocessableEntity().body(format!("{:#?}", e)),
            _ => HttpResponse::InternalServerError().body(format!("{:#?}", e)),
        };
    }
}

pub fn generate_image(req: HttpRequest, data: web::Data<AppState>, keep_ratio: bool) -> HttpResponse {
    let resource_url = &req.match_info().get("tail").unwrap().to_string();
    let resource_uri = urlencoding::decode(resource_url).unwrap();
    let width = req.match_info().get("width").unwrap_or("no-width");
    let height = req.match_info().get("height").unwrap_or("no-height");
    let output_dimensions: OutputDimensions = (width, height, keep_ratio).into();
    if let Some(response_data) = data.fetcher.lock().unwrap().serve_cache(&resource_uri) {
        let output_format = match req
            .match_info()
            .get("format")
            .unwrap_or_else(|| response_data.content_type.as_str())
            .parse::<OutputFormat>() {
            Ok(f) => f,
            Err(_) => return HttpResponse::UnprocessableEntity().body(format!("Invalid format: {}", req.match_info().get("format").unwrap_or_else(|| response_data.content_type.as_str()))),
        };
        debug!("Fetcher allowed to serve cache {:?}", response_data);
        if let Some(encoded_image) = data.encoder.lock().unwrap().serve_cache(
            &response_data.id,
            &output_dimensions,
            output_format
        ) {

            let mut response: HttpResponseBuilder = response_data.into();
            return response.content_type(encoded_image.content_type).body(encoded_image.image);
        }
    }
    let resource = match data
        .fetcher
        .lock()
        .unwrap()
        .fetch(&resource_uri) {
            Ok(r) => r,
            Err(e) => return e.into(),
    };


    info!("Received image in format: {} - size: {}", &resource.response_data.content_type, size_of_val(&*resource.content.as_slice()));
    let output_format = match req
        .match_info()
        .get("format")
        .unwrap_or_else(|| resource.response_data.content_type.as_str())
        .parse::<OutputFormat>() {
        Ok(f) => f,
        Err(_) => return HttpResponse::UnprocessableEntity().body(format!("Invalid format: {}", req.match_info().get("format").unwrap_or_else(|| resource.response_data.content_type.as_str()))),
    };

    info!("Image will be converted to: {}", output_format);

    let img = match data.decoder.lock().unwrap().decode(&resource.response_data.id, &resource) {
        Ok(img) => img,
        Err(err) => {
            return HttpResponse::UnprocessableEntity().body(format!("{:#?}", err));
        }
    };

    let resized_image_result = match output_dimensions {
        OutputDimensions::Original => {
            Result::Ok(img)
        }
        OutputDimensions::ScaledExact(width, height) => {
            data.resizer.lock().unwrap().resize_exact(&resource.response_data.id, img, (width, height))
        }
        OutputDimensions::ScaledWithRatio(width, height) => {
            data.resizer.lock().unwrap().resize(&resource.response_data.id, img, (width, height))
        }
    };

    let encoded_image = match resized_image_result {
        Ok(image) => {
            data.encoder.lock().unwrap().encode(
                &resource.response_data.id,
                image,
                &output_dimensions,
                output_format,
            ).unwrap()
        }
        Err(ResizeError::ResizeExceedsMaximumSize(maximum_size, maximum_dimensions)) => {
            return HttpResponse::BadRequest()
                .body(format!("Allowed maximum image size is: {}. Requested: {}.", maximum_size, maximum_dimensions));
        }
    };

    let mut response: HttpResponseBuilder = resource.response_data.into();
    return response.content_type(encoded_image.content_type).body(encoded_image.image);
}
