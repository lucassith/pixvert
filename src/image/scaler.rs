pub mod lanczos3_scaler;


use crate::image::{DecodedImage, EncodedImage};
use async_trait::async_trait;
use std::fmt::Error;
use crate::service_provider::Service;

pub trait ImageScalerService: Service + ImageScaler {

}

#[async_trait]
pub trait ImageScaler {
    async fn scale(&self, origin_url: &String, decoded_image: &DecodedImage, dimensions: (u32, u32)) -> Result<DecodedImage, Error>;
}