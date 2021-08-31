use std::collections::HashMap;

use bytes::Bytes;
use image::DynamicImage;

pub mod encoder;
pub mod decoder;
pub mod scaler;

#[derive(Clone)]
pub struct DecodedImage {
    pub image: DynamicImage,
    pub cache_info: HashMap<String, String>,
    pub from: String,
}

#[derive(Clone)]
pub struct EncodedImage {
    pub image: Bytes,
    pub from: String,
    pub cache_info: HashMap<String, String>,
    pub output_mime: String,
}