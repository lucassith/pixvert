extern crate image as image_crate;

use std::fmt::Error;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::cache::Cachable;
use crate::image::{DecodedImage};
use crate::image::scaler::{ImageScaler, ImageScalerService};
use crate::IMAGE_CACHE_HASH_LITERAL;
use crate::service_provider::Service;

pub struct Lanczos3ImageScaler {
    cache: Arc<Mutex<dyn Cachable<DecodedImage> + Send + Sync>>,
}

impl Lanczos3ImageScaler {
    pub fn new(cache: Arc<Mutex<dyn Cachable<DecodedImage> + Send + Sync>>) -> Lanczos3ImageScaler {
        return Lanczos3ImageScaler {
            cache
        }
    }
}

impl ImageScalerService for Lanczos3ImageScaler {

}

impl Service for Lanczos3ImageScaler {
    fn can_be_used(&self, _: &String) -> bool {
        true
    }
}

#[async_trait]
impl ImageScaler for Lanczos3ImageScaler {
    async fn scale(&self, origin_url: &String, decoded_image: &DecodedImage, dimensions: (u32, u32)) -> Result<DecodedImage, Error> {
        let image_cache_info = decoded_image.cache_info.get(
            IMAGE_CACHE_HASH_LITERAL
        );

        if let Some(cache_string) = image_cache_info {
            if let Ok(cached) = self.cache.lock().unwrap().get(&(String::from(cache_string) + &*dimensions.0.to_string() + &*dimensions.1.to_string())) {
                log::info!("Serving encoded image {} from cache", {origin_url});
                return Ok(cached);
            }
        }

        let scaled_dynamic_image = decoded_image
            .image
            .clone()
            .resize(dimensions.0, dimensions.1, image_crate::imageops::FilterType::Lanczos3);

        let scaled_image =
            DecodedImage {
                image: scaled_dynamic_image,
                from: decoded_image.from.clone(),
                cache_info: decoded_image.cache_info.clone(),
            };

        if let Some(cache_value) = image_cache_info {
            self.cache.lock().unwrap().set(cache_value.clone() + dimensions.0.to_string().as_str() + dimensions.1.to_string().as_str(), scaled_image.clone());
        }

        return Result::Ok(
            scaled_image
        )
    }
}

