use mime::Mime;
use crate::fetcher::FetchedObject;
use image::GenericImage;
use crate::image::DecodedImage;
use async_trait::async_trait;

pub mod image_create_decoder;

#[async_trait]
pub trait ImageDecoder {
    fn can_decode(mime: Mime) -> bool;
    async fn decode(origin_url: String, fetched_object: FetchedObject) -> Result<DecodedImage, DecodeError>;
}