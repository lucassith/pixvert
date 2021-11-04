use std::error::Error;
use std::sync::Mutex;
use image_crate::DynamicImage;
use image_crate::imageops::FilterType;
use crate::cache::CacheEngine;
use crate::fetcher::generate_resource_tag;
use crate::image::Image;

pub trait Resizer {
    fn resize(
        &self,
        tag: &String,
        resource: DynamicImage,
        dimensions: (usize, usize),
    ) -> Result<DynamicImage, ResizeError>;
}

#[derive(Debug)]
pub enum ResizeError {
    Unknown(String),
}


pub struct CachedResizer<'a> {
    pub cache: &'a Mutex<Box<dyn CacheEngine + Send>>,
}

impl Resizer for CachedResizer<'_> {
    fn resize(&self, tag: &String, resource: DynamicImage, dimensions: (usize, usize)) -> Result<DynamicImage, ResizeError> {
        let cached_image: Option<Vec<u8>>;
        let tag = generate_resource_tag(&format!("{} - {}x{}", tag, dimensions.0, dimensions.1));
        {
            cached_image = self.cache.lock().unwrap().get(tag.as_str());
        }
        if let Some(cached_image) = cached_image {
            let image: Image = bincode::deserialize(cached_image.as_slice()).unwrap();
            return Ok(image.into());
        }

        let mut image = resource;
        image = image.resize(dimensions.0 as u32, dimensions.1 as u32, FilterType::Lanczos3);
        let binary_image = bincode::serialize::<Image>(&image.clone().into()).unwrap();
        {
            self.cache.lock().unwrap().set(tag.as_str(), &binary_image);
        }
        return Ok(image);
    }
}

