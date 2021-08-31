use async_trait::async_trait;

use crate::fetcher::FetchedObject;
use crate::image::DecodedImage;
use crate::service_provider::Service;

pub mod image_png_jpg_decoder;

#[derive(Debug)]
pub enum DecodeError {
    InvalidInputFormat(String, String),
}

pub trait ImageDecoderService: ImageDecoder + Service {}

#[async_trait]
pub trait ImageDecoder {
    async fn decode(&self, origin_url: &String, fetched_object: FetchedObject) -> Result<DecodedImage, DecodeError>;
}