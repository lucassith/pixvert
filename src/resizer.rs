use std::sync::{Arc, RwLock};

use image_crate::DynamicImage;
use image_crate::imageops::FilterType;

use crate::cache::CacheEngine;
use crate::config::Config;
use crate::fetcher::generate_resource_tag;
use crate::image::Image;
use crate::resizer::ResizeError::ResizeExceedsMaximumSize;

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
    ResizeExceedsMaximumSize(usize, usize),
}


pub struct CachedResizer {
    pub cache: Arc<RwLock<Box<dyn CacheEngine + Send + Sync>>>,
    pub config: Config,
}

pub(self) fn resize(
    resource: DynamicImage,
    dimensions: (usize, usize),
    maximum_size: usize,
    exact: bool
) -> Result<DynamicImage, ResizeError> {
    let maximum_dimensions = dimensions.0 * dimensions.1;
    if maximum_dimensions > maximum_size {
        return Result::Err(ResizeExceedsMaximumSize(maximum_size, maximum_dimensions));
    }
    let mut image = resource;
    if exact {
        image = image.resize_exact(dimensions.0 as u32, dimensions.1 as u32, FilterType::Lanczos3);
    } else {
        image = image.resize(dimensions.0 as u32, dimensions.1 as u32, FilterType::Lanczos3);
    }
    return Result::Ok(image);
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
        let image = resize(resource, dimensions, self.config.maximum_image_size, false)?;
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

        let image = resize(resource, dimensions, self.config.maximum_image_size, true)?;
        let binary_image = bincode::serialize::<Image>(&image.clone().into()).unwrap();
        {
            self.cache.write().unwrap().set(tag.as_str(), &binary_image);
        }
        return Ok(image);
    }
}

