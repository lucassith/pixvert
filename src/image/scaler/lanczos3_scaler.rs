extern crate image as image_crate;

use crate::image::{EncodedImage, DecodedImage};
use crate::cache::Cachable;
use std::sync::{Mutex, Arc};
use std::fmt::Error;
use webp::Encoder;
use async_trait::async_trait;
use crate::service_provider::Service;
use crate::image::scaler::{ImageScaler, ImageScalerService};
use sha2::{Sha224, Digest};


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
        let mut hasher = Sha224::new();
        hasher.update(&decoded_image.image.as_bytes());
        hasher.update(&decoded_image.from.to_string());
        hasher.update(format!("{}x{}", dimensions.0, dimensions.1));
        let hash = hasher.finalize();
        if let Ok(cached) = self.cache.lock().unwrap().get(&String::from_utf8_lossy(&*hash).to_string()) {
            log::info!("Serving scaled image {} from cache", {origin_url});
            return Ok(cached);
        }

        let scaled_dynamic_image = decoded_image
            .image
            .clone()
            .resize_exact(dimensions.0, dimensions.1, image_crate::imageops::FilterType::Lanczos3);

        let scaled_image =
            DecodedImage{
                image: scaled_dynamic_image,
                from: decoded_image.from.clone(),
            };


        self.cache.lock().unwrap().set(String::from_utf8_lossy(&*hash).to_string(), scaled_image.clone());

        return Result::Ok(
            scaled_image
        )
    }
}

