mod color_blindness;
mod image_converter;

use crate::tool::Tool;

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(color_blindness::ColorBlindness::default()),
        Box::new(image_converter::ImageConverter::default()),
    ]
}
