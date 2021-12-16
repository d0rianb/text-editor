use std::fs;
use std::rc::Rc;
use speedy2d::font::{Font as S2DFont, FormattedTextBlock, TextLayout, TextOptions};

const FONT_SIZE: u32 = 16;

#[derive(Debug, Clone)]
pub(crate) struct Font {
    pub name: String,
    pub char_width: f32,
    pub char_height: f32,
    pub s2d_font: S2DFont
}

impl Font {
    pub fn new(src: &str) -> Self {
        let font_file_content = fs::read(src).unwrap();
        let s2d_font = S2DFont::new(&font_file_content).unwrap();
        let font_layout = s2d_font.layout_text("a", 2.0*FONT_SIZE as f32, TextOptions::default());
        Self {
            name: src.to_string(),
            char_width: font_layout.width().round(),
            char_height: font_layout.height().round(),
            s2d_font
        }
    }

    pub fn layout_text(&self, text: &str) -> Rc<FormattedTextBlock> {
        self.s2d_font.layout_text(text, 2.0*FONT_SIZE as f32, TextOptions::default())
    }
}
