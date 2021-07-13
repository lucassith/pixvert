use image::{GenericImage, ImageFormat};
use crate::cache::Cachable;
use crate::image::decoder::{ImageDecoder, DecodeError};
use std::sync::{Mutex, Arc};
use mime::Mime;
use crate::fetcher::FetchedObject;
use crate::image::DecodedImage;
use image::io::Reader;
use std::io::Cursor;
use async_trait::async_trait;
use bytes::Buf;

pub struct ImageCreateDecoder {
    cache: Arc<Mutex<dyn Cachable<DecodedImage> + Send + Sync>>,
}

#[async_trait]
impl ImageDecoder for ImageCreateDecoder {
    fn can_decode(mime: Mime) -> bool {
        if mime.eq(&mime::IMAGE_JPEG) || mime.eq(&mime::IMAGE_PNG) {
            return true;
        }
        return false;
    }

    async fn decode(origin_url: String, fetched_object: FetchedObject) -> Result<DecodedImage, DecodeError> {
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

