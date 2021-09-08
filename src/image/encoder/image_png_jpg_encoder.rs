use std::fmt::{Error, Display, Formatter};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::cache::Cachable;
use crate::image::{DecodedImage, EncodedImage};
use crate::image::encoder::{ImageEncoder, ImageEncoderService};
use crate::IMAGE_CACHE_HASH_LITERAL;
use crate::service_provider::Service;
use image::{ImageOutputFormat};

pub enum ImagePngJpgEncoderType {
    JPG,
    PNG
}

impl Display for ImagePngJpgEncoderType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ImagePngJpgEncoderType::PNG => write!(f, "{}", "PNG"),
            ImagePngJpgEncoderType::JPG => write!(f, "{}", "JPG"),
        }
    }
}

impl PartialEq<String> for ImagePngJpgEncoderType {
    fn eq(&self, other: &String) -> bool {
        other == &self.to_mime()
    }
}

impl ImagePngJpgEncoderType {
    pub fn to_mime(&self) -> String {
        match self {
            ImagePngJpgEncoderType::PNG => mime::IMAGE_PNG.to_string(),
            ImagePngJpgEncoderType::JPG => mime::IMAGE_JPEG.to_string()
        }
    }
}

pub struct ImagePngJpgEncoder {
    cache: Arc<Mutex<dyn Cachable<EncodedImage> + Send + Sync>>,
    output_format: ImagePngJpgEncoderType,
}

impl ImagePngJpgEncoder {
    pub fn new(
        cache: Arc<Mutex<dyn Cachable<EncodedImage> + Send + Sync>>,
        output_format: ImagePngJpgEncoderType
    ) -> ImagePngJpgEncoder {
        return ImagePngJpgEncoder {
            cache,
            output_format
        };
    }
}

impl ImageEncoderService for ImagePngJpgEncoder {}

impl Service for ImagePngJpgEncoder {
    fn can_be_used(&self, resource: &String) -> bool {
        self.output_format.eq(resource)
    }
}

#[async_trait]
impl ImageEncoder for ImagePngJpgEncoder {
    async fn encode(&self, origin_url: &String, decoded_image: DecodedImage, quality: f32) -> Result<EncodedImage, Error> {
        let image_cache_info = decoded_image.cache_info.get(
            IMAGE_CACHE_HASH_LITERAL
        );

        let md5_url = format!("{:x}_{}", md5::compute(origin_url), self.output_format);

        if let Some(cache_string) = image_cache_info {
            if let Ok(cached) = self.cache.lock().unwrap().get(&(String::from(cache_string) + &*md5_url.clone())) {
                log::info!("Serving encoded image {} from cache", {origin_url});
                return Ok(cached);
            }
        }

        let mut bytes: Vec<u8> = Vec::new();
        match self.output_format {
            ImagePngJpgEncoderType::JPG => {
                decoded_image.image.write_to(
                    &mut bytes, ImageOutputFormat::Jpeg(quality as u8),
                ).unwrap();
            }
            ImagePngJpgEncoderType::PNG => {
                decoded_image.image.write_to(
                    &mut bytes, ImageOutputFormat::Png,
                ).unwrap();
            }
        }


        let encoded_image =
            EncodedImage {
                image: bytes::Bytes::from(bytes),
                from: decoded_image.from,
                output_mime: self.output_format.to_mime().clone(),
                cache_info: decoded_image.cache_info.clone(),
            };


        if let Some(cache_value) = image_cache_info {
            self.cache.lock().unwrap().set(cache_value.clone() + md5_url.as_str(), encoded_image.clone());
        }

        return Result::Ok(
            encoded_image
        );
    }
}

