use image::{DynamicImage};
use mime::Mime;
use bytes::Bytes;
use std::collections::HashMap;

pub mod encoder;
pub mod decoder;
pub mod scaler;

#[derive(Clone)]
pub struct DecodedImage {
    pub image: DynamicImage,
    pub cache_info: HashMap<String, String>,
    pub from: Mime,
}

#[derive(Clone)]
pub struct EncodedImage {
    pub image: Bytes,
    pub from: Mime,
    pub cache_info: HashMap<String, String>,
    pub output_mime: String,
}