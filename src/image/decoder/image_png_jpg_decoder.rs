use image::{ ImageFormat};
use crate::cache::Cachable;
use crate::image::decoder::{ImageDecoder, DecodeError, ImageDecoderService};
use std::sync::{Mutex, Arc};
use crate::fetcher::FetchedObject;
use crate::image::DecodedImage;
use image::io::Reader;
use std::io::Cursor;
use async_trait::async_trait;
use crate::service_provider::Service;
use sha2::{Sha224, Digest};


pub struct ImagePngJpgDecoder {
    cache: Arc<Mutex<dyn Cachable<DecodedImage> + Send + Sync>>,
}

impl ImagePngJpgDecoder {
    pub fn new(cache: Arc<Mutex<dyn Cachable<DecodedImage> + Send + Sync>>) -> ImagePngJpgDecoder {
        return ImagePngJpgDecoder{
            cache
        }
    }
}

impl ImageDecoderService for ImagePngJpgDecoder {

}

impl Service for ImagePngJpgDecoder {
    fn can_be_used(&self, resource: &String) -> bool {
        return resource.eq(&mime::IMAGE_JPEG.to_string()) || resource.eq(&mime::IMAGE_PNG.to_string());
    }
}

#[async_trait]
impl ImageDecoder for ImagePngJpgDecoder {
    async fn decode(&self, origin_url: &String, fetched_object: FetchedObject) -> Result<DecodedImage, DecodeError> {
        let format = if fetched_object.mime.eq(&mime::IMAGE_JPEG) {
            ImageFormat::Jpeg
        } else {
            ImageFormat::Png
        };
        let mut hasher = Sha224::new();
        hasher.update(&fetched_object.bytes);
        hasher.update(fetched_object.mime.clone().to_string());
        let hash = hasher.finalize();
        if let Ok(cached) = self.cache.lock().unwrap().get(&String::from_utf8_lossy(&*hash).to_string()) {
            log::info!("Serving decoded image {} from cache", {origin_url});
            return Ok(cached);
        }

        let mut reader = Reader::new(Cursor::new(
            fetched_object.bytes.to_vec()
        ));
        reader.set_format(format);

        let decoded_image =
            DecodedImage{
                image: reader.decode().unwrap(),
                from: fetched_object.mime,
            };


        self.cache.lock().unwrap().set(String::from_utf8_lossy(&*hash).to_string(), decoded_image.clone());

        return Result::Ok(
            decoded_image
        )
    }
}

