use crate::image::{EncodedImage, DecodedImage};
use crate::cache::Cachable;
use std::sync::{Mutex, Arc};
use crate::image::encoder::{ImageEncoder, ImageEncoderService};
use std::fmt::Error;
use webp::Encoder;
use async_trait::async_trait;
use crate::service_provider::Service;

pub struct ImageWebpEncoder {
    cache: Arc<Mutex<dyn Cachable<EncodedImage> + Send + Sync>>,
}

impl ImageWebpEncoder {
    pub fn new(cache: Arc<Mutex<dyn Cachable<EncodedImage> + Send + Sync>>) -> ImageWebpEncoder {
        return ImageWebpEncoder {
            cache
        }
    }
}

impl ImageEncoderService for ImageWebpEncoder {

}

impl Service for ImageWebpEncoder {
    fn can_be_used(&self, resource: &String) -> bool {
        resource.eq("image/webp")
    }
}

#[async_trait]
impl ImageEncoder for ImageWebpEncoder {
    async fn encode(&self, origin_url: &String, decoded_image: DecodedImage) -> Result<EncodedImage, Error> {
        let mut hasher = Sha224::new();
        hasher.update(&decoded_image.image.as_bytes());
        hasher.update(&decoded_image.from.to_string());
        let hash = hasher.finalize();
        if let Ok(cached) = self.cache.lock().unwrap().get(&String::from_utf8_lossy(&*hash).to_string()) {
            log::info!("Serving encoded webp image {} from cache", {origin_url});
            return Ok(cached);
        }
        let encoder = Encoder::from_image(
            &decoded_image.image
        );
        let encoded_image_buffer = encoder.encode(20f32);
        let encoded_image = EncodedImage{
            image: bytes::Bytes::from(encoded_image_buffer.to_vec()),
            from: decoded_image.from,
            output_mime: String::from("image/webp")
        };

        self.cache.lock().unwrap().set(String::from_utf8_lossy(&*hash).to_string(), encoded_image.clone());

        Result::Ok(encoded_image)
    }
}

