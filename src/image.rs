use image_crate::{DynamicImage, GenericImageView, RgbaImage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Image {
    pub image: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl From<DynamicImage> for Image {
    fn from(img: DynamicImage) -> Self {
        return Image {
            image: img.to_rgba8().as_raw().clone(),
            width: img.width(),
            height: img.height(),
        };
    }
}

impl From<Image> for DynamicImage {
    fn from(img: Image) -> Self {
        return DynamicImage::ImageRgba8(
            RgbaImage::from_raw(img.width, img.height, img.image).unwrap()
        );
    }
}

impl Default for Image {
    fn default() -> Self {
        Image { image: Vec::default(), width: 0, height: 0 }
    }
}


#[cfg(test)]
mod tests {
    use image_crate::{DynamicImage, GenericImage, Rgba};

    use crate::image;

    #[test]
    fn serialize_image_struct() {
        let mut img: DynamicImage = DynamicImage::new_rgb8(2, 3).into();
        img.put_pixel(1, 1, Rgba([255, 0, 100, 255]));

        let image_struct: image::Image = img.into();

        let s = bincode::serialize(&image_struct).unwrap();
        let expected_bincode: Vec<u8> = Vec::from([24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 100, 255, 0, 0, 0, 255, 0, 0, 0, 255, 2, 0, 0, 0, 3, 0, 0, 0]);

        assert_eq!(expected_bincode, s);
    }

    #[test]
    fn deserialize_image_struct() {
        let image_bincode: &[u8] = &[24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 100, 255, 0, 0, 0, 255, 0, 0, 0, 255, 2, 0, 0, 0, 3, 0, 0, 0];

        let image: image::Image = bincode::deserialize(image_bincode).unwrap();

        assert_eq!(image.width, 2);
        assert_eq!(image.height, 3);
        assert_eq!(image.image, Vec::from([0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 100, 255, 0, 0, 0, 255, 0, 0, 0, 255]))
    }
}
