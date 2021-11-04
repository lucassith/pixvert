use std::io::Cursor;
use std::sync::Mutex;
use image_crate::{DynamicImage, ImageFormat};
use crate::cache::CacheEngine;
use image_crate::io::Reader as ImageReader;
use crate::fetcher::{generate_resource_tag, Resource};
use crate::image::Image;

pub trait ImageDecoder {
    fn decode(&self, tag: &String, resource: Resource) -> DynamicImage;
}

pub struct CachedImageDecoder<'a> {
    pub cache: &'a Mutex<Box<dyn CacheEngine + Send>>,
}

impl ImageDecoder for CachedImageDecoder<'_> {
    fn decode(&self, tag: &String, resource: Resource) -> DynamicImage {

        let tag = generate_resource_tag(&format!("Image Decoder {}", tag));

        if let Some(dynamic_image_bytes) = self.cache.lock().unwrap().get(&tag) {
            return bincode::deserialize::<Image>(&dynamic_image_bytes).unwrap().into();
        }

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
        let img = reader.decode().unwrap();

        self.cache.lock().unwrap().set(&tag, &bincode::serialize::<Image>(&img.clone().into()).unwrap());

        return img;
    }
}
