use image::{DynamicImage};
use mime::Mime;

pub mod encoder;
pub mod decoder;

#[derive(Clone)]
pub struct DecodedImage {
    image: DynamicImage,
    from: Mime,
}