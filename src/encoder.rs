use std::fmt::{Display, Formatter};
use std::num::{ParseFloatError, ParseIntError};
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use image_crate::{DynamicImage, GenericImageView, ImageOutputFormat};
use jpegxl_rs::encode::{EncoderResult, EncoderSpeed, JxlEncoder};
use jpegxl_rs::EncodeError as JpegXlEncodeError;
use jpegxl_rs::encoder_builder;
use log::info;
use serde::{Deserialize, Serialize};

use crate::cache::CacheEngine;
use crate::fetcher::generate_resource_tag;

#[derive(Debug)]
pub enum OutputFormat {
    Jpeg(u8),
    Png,
    JpegXl(f32),
    JpegXlLoseless,
    WebpLoseless,
    Webp(f32),
    Bmp,
}

impl FromStr for OutputFormat {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("png") { return Ok(OutputFormat::Png); }
        if s.starts_with("bmp") { return Ok(OutputFormat::Bmp); }
        if s.starts_with("jpegxl") {
            let (_, quality) = s.split_at(6);
            return if quality != "" {
                let quality_f32: f32 = quality.parse()?;
                if quality_f32 < 0.01 || quality_f32 > 15.0 {
                    return Err(ParseError::QualityOutOfRange(String::from("JpegXL must be between 0.01 (best) to 15 (worst)")));
                }
                Ok(OutputFormat::JpegXl(quality_f32))
            } else {
                Ok(OutputFormat::JpegXlLoseless)
            };
        }
        if s.starts_with("jpeg") {
            let (_, quality) = s.split_at(4);
            return if quality != "" {
                let quality_u8: u8 = quality.parse()?;
                if quality_u8 > 100 {
                    return Err(ParseError::QualityOutOfRange(String::from("JpegXL must be between 0 (worst) to 100 (best)")));
                }
                Ok(OutputFormat::Jpeg(quality_u8))
            } else {
                Ok(OutputFormat::Jpeg(90))
            };
        }
        if s.starts_with("webp") {
            let (_, quality) = s.split_at(4);
            return if quality != "" {
                let quality_f32: f32 = quality.parse()?;
                if quality_f32 > 100.0 {
                    return Err(ParseError::QualityOutOfRange(String::from("JpegXL must be between 0 (worst) to 100 (best)")));
                }
                Ok(OutputFormat::Webp(quality_f32))
            } else {
                Ok(OutputFormat::WebpLoseless)
            };
        }
        if s == "image/webp" { return Ok(OutputFormat::WebpLoseless); }
        if s == "image/png" { return Ok(OutputFormat::Png); }
        if s == "image/bmp" { return Ok(OutputFormat::Bmp); }
        if s == "image/jpeg" { return Ok(OutputFormat::Jpeg(90)); }
        if s == "image/jpegxl" { return Ok(OutputFormat::JpegXl(90f32)); }
        return Err(ParseError::InvalidFormat(s.to_string()));
    }
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Png => write!(f, "image/png"),
            OutputFormat::WebpLoseless => write!(f, "image/webp - loseless"),
            OutputFormat::Jpeg(q) => write!(f, "image/jpeg - quality: {}", q),
            OutputFormat::Webp(q) => write!(f, "image/webp - quality: {}", q),
            OutputFormat::JpegXl(q) => write!(f, "image/jxl - quality: {}", q),
            OutputFormat::JpegXlLoseless => write!(f, "image/jxl - loseless"),
            OutputFormat::Bmp => write!(f, "image/bmp"),
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    InvalidIntQuality(ParseIntError),
    InvalidFloatQuality(ParseFloatError),
    QualityOutOfRange(String),
    InvalidFormat(String),
}

impl From<ParseIntError> for ParseError {
    fn from(e: ParseIntError) -> Self {
        ParseError::InvalidIntQuality(e)
    }
}

impl From<ParseFloatError> for ParseError {
    fn from(e: ParseFloatError) -> Self {
        ParseError::InvalidFloatQuality(e)
    }
}

#[derive(Debug)]
pub enum EncodingError {
    JpegXlEncodeError(String),
    Unknown(String),
}

impl From<JpegXlEncodeError> for EncodingError {
    fn from(e: JpegXlEncodeError) -> Self {
        return EncodingError::JpegXlEncodeError(format!("{:#?}", e));
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EncodedImage {
    pub content_type: String,
    pub image: Vec<u8>,
}

pub trait ImageEncoder {
    fn encode(&self, tag: &String, resource: DynamicImage, output_format: OutputFormat) -> Result<EncodedImage, EncodingError>;
}

pub struct AllInOneCachedImageEncoder {
    pub cache: Arc<RwLock<Box<dyn CacheEngine + Send + Sync>>>,
}

impl ImageEncoder for AllInOneCachedImageEncoder {
    fn encode(&self, tag: &String, resource: DynamicImage, output_format: OutputFormat) -> Result<EncodedImage, EncodingError> {
        let mut image: Vec<u8> = Vec::default();
        let content_type: String;

        let tag = generate_resource_tag(&format!("{} - {} {}x{}", tag, output_format, resource.width(), resource.height()));
        if let Some(cached_encoded_image) = self.cache.read().unwrap().get(&tag) {
            info!("Serving {} {} from cache.", tag, output_format);
            return Ok(bincode::deserialize(cached_encoded_image.as_slice()).unwrap());
        }

        match output_format {
            OutputFormat::Jpeg(quality) => {
                resource.write_to(&mut image, ImageOutputFormat::Jpeg(quality));
                content_type = mime::IMAGE_JPEG.to_string();
            }
            OutputFormat::Png => {
                resource.write_to(&mut image, ImageOutputFormat::Png);
                content_type = mime::IMAGE_PNG.to_string();
            }
            OutputFormat::Bmp => {
                resource.write_to(&mut image, ImageOutputFormat::Bmp);
                content_type = mime::IMAGE_BMP.to_string();
            }
            OutputFormat::JpegXl(quality) => {
                let mut encoder: JxlEncoder = encoder_builder()
                    .lossless(true)
                    .speed(EncoderSpeed::Wombat)
                    .build()?;
                encoder.lossless = false;
                encoder.quality = quality;
                let result: EncoderResult<u8> = encoder.encode(&resource.as_bytes(), resource.width(), resource.height()).unwrap();
                image = result.data;
                content_type = String::from("image/jxl");
            }
            OutputFormat::JpegXlLoseless => {
                let mut encoder: JxlEncoder = encoder_builder()
                    .lossless(true)
                    .speed(EncoderSpeed::Tortoise)
                    .build()?;
                encoder.lossless = true;
                let result: EncoderResult<u8> = encoder.encode(&resource.as_bytes(), resource.width(), resource.height()).unwrap();
                image = result.data;
                content_type = String::from("image/jxl");
            }
            OutputFormat::WebpLoseless => {
                let mut encoder = webp::Encoder::from_image(&resource).unwrap();
                image = encoder.encode_lossless().to_vec();
                content_type = String::from("image/webp")
            }
            OutputFormat::Webp(quality) => {
                let mut encoder = webp::Encoder::from_image(&resource).unwrap();
                image = encoder.encode(quality).to_vec();
                content_type = String::from("image/webp")
            }
        }
        let encoded_image = EncodedImage {
            image,
            content_type,
        };

        info!("Saving {} {} to cache.", tag, output_format);
        self.cache.write().unwrap().set(&tag, &bincode::serialize(&encoded_image.clone()).unwrap());

        return Ok(encoded_image);
    }
}
