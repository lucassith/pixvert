use std::io::Cursor;
use std::sync::{Arc, Mutex, RwLock};

use image_crate::{DynamicImage, ImageFormat};
use image_crate::io::Reader as ImageReader;

use crate::cache::CacheEngine;
use crate::fetcher::{generate_resource_tag, Resource};
use crate::image::Image;

pub trait ImageDecoder {
    fn decode(&self, tag: &String, resource: Resource) -> Result<DynamicImage, DecodeError>;
}

#[derive(Debug)]
pub enum DecodeError {
    UnknownFormat(String),
    MismatchedFormat,
}

pub struct CachedImageDecoder {
    pub cache: Arc<RwLock<Box<dyn CacheEngine + Send + Sync>>>,
}

impl ImageDecoder for CachedImageDecoder {
    fn decode(&self, tag: &String, resource: Resource) -> Result<DynamicImage, DecodeError> {
        let tag = generate_resource_tag(&format!("Image Decoder {}", tag));

        if let Some(dynamic_image_bytes) = self.cache.read().unwrap().get(&tag) {
            return Ok(bincode::deserialize::<Image>(&dynamic_image_bytes).unwrap().into());
        }

        let mut img: DynamicImage;

        if resource.content_type.as_str() == "image/webp" {
            let decoder = webp::Decoder::new(resource.content.as_slice());
            img = match decoder.decode() {
                Some(image) => image.to_image(),
                None => return Err(DecodeError::MismatchedFormat),
            };
        } else {
            let mut reader = ImageReader::new(Cursor::new(
                resource.content
            ));
            match resource.content_type.as_str() {
                "image/jpeg" => {
                    reader.set_format(ImageFormat::Jpeg);
                }
                "image/png" => {
                    reader.set_format(ImageFormat::Png);
                }
                "image/bmp" => {
                    reader.set_format(ImageFormat::Bmp);
                }
                "image/x-tga" | "image/x-targa" => {
                    reader.set_format(ImageFormat::Tga);
                }
                _ => {
                    reader = reader.with_guessed_format().unwrap();
                }
            }

            if let Ok(image) = reader.decode() {
                img = image;
            } else {
                return Err(DecodeError::UnknownFormat(resource.content_type.clone()));
            }
        }

        self.cache.write().unwrap().set(&tag, &bincode::serialize::<Image>(&img.clone().into()).unwrap());
        return Ok(img);
    }
}
