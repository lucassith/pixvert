use crate::fetcher::FetchedObject;
use crate::image::DecodedImage;
use async_trait::async_trait;
use crate::service_provider::Service;

pub mod image_png_jpg_decoder;

#[derive(Debug)]
pub enum DecodeError {
    InvalidInputFormat(String, String),
}

pub trait ImageDecoderService: ImageDecoder + Service {

}

#[async_trait]
pub trait ImageDecoder {
    fn can_decode(&self, mime: &String) -> bool;
    async fn decode(&self, origin_url: &String, fetched_object: FetchedObject) -> Result<DecodedImage, DecodeError>;
}