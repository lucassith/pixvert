use std::io::Cursor;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use image::ImageFormat;
use image::io::Reader;

use crate::cache::Cachable;
use crate::fetcher::FetchedObject;
use crate::image::DecodedImage;
use crate::image::decoder::{DecodeError, ImageDecoder, ImageDecoderService};
use crate::IMAGE_CACHE_HASH_LITERAL;
use crate::service_provider::Service;

pub struct ImagePngJpgDecoder {
    cache: Arc<Mutex<dyn Cachable<DecodedImage> + Send + Sync>>,
}

impl ImagePngJpgDecoder {
    pub fn new(cache: Arc<Mutex<dyn Cachable<DecodedImage> + Send + Sync>>) -> ImagePngJpgDecoder {
        return ImagePngJpgDecoder {
            cache
        };
    }
}

impl ImageDecoderService for ImagePngJpgDecoder {}

impl Service for ImagePngJpgDecoder {
    fn can_be_used(&self, resource: &String) -> bool {
        resource.eq(&mime::IMAGE_JPEG.to_string()) || resource.eq(&mime::IMAGE_PNG.to_string())
    }
}

#[async_trait]
impl ImageDecoder for ImagePngJpgDecoder {
    async fn decode(&self, origin_url: &String, fetched_object: &FetchedObject) -> Result<DecodedImage, DecodeError> {
        let format = if fetched_object.mime.eq(&mime::IMAGE_JPEG.to_string()) {
            ImageFormat::Jpeg
        } else {
            ImageFormat::Png
        };

        let image_cache_info = fetched_object.cache_info.get(
            IMAGE_CACHE_HASH_LITERAL
        );

        log::trace!("fetched object cache: {:#?}", fetched_object.cache_info);

        if let Some(cache_string) = image_cache_info {
            log::trace!("Object cache string: {}", cache_string);
            if let Ok(cached) = self.cache.lock().unwrap().get(cache_string) {
                log::info!("Serving encoded image {} from cache", {origin_url});
                return Ok(cached);
            }
        }


        let mut reader = Reader::new(Cursor::new(
            fetched_object.bytes.to_vec()
        ));
        reader.set_format(format);

        let decoded_image =
            DecodedImage {
                image: reader.decode().unwrap(),
                from: fetched_object.mime.clone(),
                cache_info: fetched_object.cache_info.clone(),
            };


        if let Some(cache_value) = image_cache_info {
            self.cache.lock().unwrap().set(cache_value.clone(), decoded_image.clone());
        }

        return Result::Ok(
            decoded_image
        );
    }
}

