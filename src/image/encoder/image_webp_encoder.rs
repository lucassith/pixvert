use std::fmt::Error;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use webp::Encoder;

use crate::cache::Cachable;
use crate::image::{DecodedImage, EncodedImage};
use crate::image::encoder::{ImageEncoder, ImageEncoderService};
use crate::IMAGE_CACHE_HASH_LITERAL;
use crate::service_provider::Service;

pub struct ImageWebpEncoder {
    cache: Arc<Mutex<dyn Cachable<EncodedImage> + Send + Sync>>,
}

impl ImageWebpEncoder {
    pub fn new(cache: Arc<Mutex<dyn Cachable<EncodedImage> + Send + Sync>>) -> ImageWebpEncoder {
        return ImageWebpEncoder {
            cache
        };
    }
}

impl ImageEncoderService for ImageWebpEncoder {}

impl Service for ImageWebpEncoder {
    fn can_be_used(&self, resource: &String) -> bool {
        resource.eq("image/webp")
    }
}

#[async_trait]
impl ImageEncoder for ImageWebpEncoder {
    async fn encode(&self, origin_url: &String, decoded_image: DecodedImage, quality: f32) -> Result<EncodedImage, Error> {
        let image_cache_info = decoded_image.cache_info.get(
            IMAGE_CACHE_HASH_LITERAL
        );

        let md5_url = format!("{:x}", md5::compute(origin_url));

        if let Some(cache_string) = image_cache_info {
            if let Ok(cached) = self.cache.lock().unwrap().get(&(String::from(cache_string) + &*md5_url.clone())) {
                log::info!("Serving encoded image {} from cache", {origin_url});
                return Ok(cached);
            }
        }


        let encoder = Encoder::from_image(
            &decoded_image.image
        );
        let encoded_image_buffer = encoder.encode(quality);
        let encoded_image = EncodedImage {
            image: bytes::Bytes::from(encoded_image_buffer.to_vec()),
            from: decoded_image.from,
            output_mime: String::from("image/webp"),
            cache_info: decoded_image.cache_info.clone(),
        };

        if let Some(cache_value) = image_cache_info {
            self.cache.lock().unwrap().set(cache_value.clone() + md5_url.as_str(), encoded_image.clone());
        }

        Result::Ok(encoded_image)
    }
}

