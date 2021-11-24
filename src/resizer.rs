use std::sync::{Arc, RwLock};

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
    fn resize_exact(
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


pub struct CachedResizer {
    pub cache: Arc<RwLock<Box<dyn CacheEngine + Send + Sync>>>,
}

impl Resizer for CachedResizer {
    fn resize(&self, tag: &String, resource: DynamicImage, dimensions: (usize, usize)) -> Result<DynamicImage, ResizeError> {
        let cached_image: Option<Vec<u8>>;
        let tag = generate_resource_tag(&format!("{} - {}x{}", tag, dimensions.0, dimensions.1));
        {
            cached_image = self.cache.read().unwrap().get(tag.as_str());
        }
        if let Some(cached_image) = cached_image {
            let image: Image = bincode::deserialize(cached_image.as_slice()).unwrap();
            return Ok(image.into());
        }

        let mut image = resource;
        image = image.resize(dimensions.0 as u32, dimensions.1 as u32, FilterType::Lanczos3);
        let binary_image = bincode::serialize::<Image>(&image.clone().into()).unwrap();
        {
            self.cache.write().unwrap().set(tag.as_str(), &binary_image);
        }
        return Ok(image);
    }

    fn resize_exact(&self, tag: &String, resource: DynamicImage, dimensions: (usize, usize)) -> Result<DynamicImage, ResizeError> {
        let cached_image: Option<Vec<u8>>;
        let tag = generate_resource_tag(&format!("{} - {}x{} exact", tag, dimensions.0, dimensions.1));
        {
            cached_image = self.cache.read().unwrap().get(tag.as_str());
        }
        if let Some(cached_image) = cached_image {
            let image: Image = bincode::deserialize(cached_image.as_slice()).unwrap();
            return Ok(image.into());
        }

        let mut image = resource;
        image = image.resize_exact(dimensions.0 as u32, dimensions.1 as u32, FilterType::Lanczos3);
        let binary_image = bincode::serialize::<Image>(&image.clone().into()).unwrap();
        {
            self.cache.write().unwrap().set(tag.as_str(), &binary_image);
        }
        return Ok(image);
    }
}

