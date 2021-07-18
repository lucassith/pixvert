use image::{ ImageFormat};
use crate::cache::Cachable;
use crate::image::decoder::{ImageDecoder, DecodeError};
use std::sync::{Mutex, Arc};
use crate::fetcher::FetchedObject;
use crate::image::DecodedImage;
use image::io::Reader;
use std::io::Cursor;
use async_trait::async_trait;


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

#[async_trait]
impl ImageDecoder for ImagePngJpgDecoder {
    fn can_decode(&self, mime: &String) -> bool {
        mime.eq(&mime::IMAGE_JPEG.to_string()) || mime.eq(&mime::IMAGE_PNG.to_string())
    }

    async fn decode(&self, origin_url: &String, fetched_object: FetchedObject) -> Result<DecodedImage, DecodeError> {
        let format = if fetched_object.mime.eq(&mime::IMAGE_JPEG) {
            ImageFormat::Jpeg
        } else {
            ImageFormat::Png
        };
        let mut reader = Reader::new(Cursor::new(
            fetched_object.bytes.to_vec()
        ));
        reader.set_format(format);

        let decoded_image = reader.decode().unwrap();

        return Result::Ok(
            DecodedImage{
                image: decoded_image,
                from: fetched_object.mime,
            }
        )
    }
}

