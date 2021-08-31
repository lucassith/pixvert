use std::fmt::Error;

use async_trait::async_trait;

use crate::image::{DecodedImage};
use crate::service_provider::Service;

pub mod lanczos3_scaler;


pub trait ImageScalerService: Service + ImageScaler {}

#[async_trait]
pub trait ImageScaler {
    async fn scale(&self, origin_url: &String, decoded_image: &DecodedImage, dimensions: (u32, u32)) -> Result<DecodedImage, Error>;
}