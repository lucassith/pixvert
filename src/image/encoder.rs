use std::fmt::Error;

use async_trait::async_trait;

use crate::image::{DecodedImage, EncodedImage};
use crate::service_provider::Service;

pub mod image_webp_encoder;

pub trait ImageEncoderService: Service + ImageEncoder {}

#[async_trait]
pub trait ImageEncoder {
    async fn encode(&self, origin_url: &String, decoded_image: DecodedImage) -> Result<EncodedImage, Error>;
}