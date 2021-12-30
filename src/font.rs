use speedy2d::dimen::Vector2;
use speedy2d::font::{Font as S2DFont, FormattedTextBlock, TextAlignment, TextLayout, TextOptions};
use std::fs;
use std::rc::Rc;
use crate::editor::EDITOR_PADDING;

const FONT_SIZE: u32 = 16;

#[derive(Debug, Clone)]
pub(crate) struct Font {
    pub name: String,
    pub char_width: f32,
    pub char_height: f32,
    pub editor_size: Vector2<f32>,
    pub s2d_font: S2DFont,
}

impl Font {
    pub fn new(src: &str, editor_width: f32, editor_height: f32) -> Self {
        let font_file_content = include_bytes!("../resources/font/CourierRegular.ttf"); //fs::read(src).unwrap();
        let s2d_font = S2DFont::new(font_file_content).unwrap();
        let font_layout = s2d_font.layout_text("a", 2.0 * FONT_SIZE as f32, TextOptions::default());
        Self {
            name: src.to_string(),
            char_width: font_layout.width(),
            char_height: font_layout.height(),
            editor_size: (editor_width, editor_height).into(), // arbitrary
            s2d_font,
        }
    }

    pub fn format(&self, text: &str) -> String {
        text
            .replace("-->" ,"\u{2192}")
            .replace("->" ,"\u{2192}")
            .replace("<--" ,"\u{2190}")
            .replace("<-" ,"\u{2190}")
            .replace("!=" ,"\u{2260}")
            .replace("<=" ,"\u{2264}")
            .replace(">=" ,"\u{2265}")
    }

    pub fn layout_text(&self, text: &str) -> Rc<FormattedTextBlock> {
        let text_layout_options = TextOptions::default().with_wrap_to_width(
            self.editor_size.x - 2. * EDITOR_PADDING,
            TextAlignment::Left,
        );
        let escaped_text = self.format(text)
            .replace('\t', "  ")// Just for rendering
            .replace(" " ,"\u{a0}");  // Just for rendering
        self.s2d_font.layout_text(&escaped_text, 2.0 * FONT_SIZE as f32, text_layout_options)
    }

    pub fn on_resize(&mut self, size: Vector2<u32>) {
        self.editor_size = Vector2::new(
            size.x as f32,
            size.y as f32,
        );
    }
}
