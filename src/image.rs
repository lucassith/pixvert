use image::{DynamicImage};
use mime::Mime;
use bytes::Bytes;

pub mod encoder;
pub mod decoder;

#[derive(Clone)]
pub struct DecodedImage {
    pub image: DynamicImage,
    pub from: Mime,
}

#[derive(Clone)]
pub struct EncodedImage {
    pub image: Bytes,
    pub from: Mime,
    pub output_mime: String,
}