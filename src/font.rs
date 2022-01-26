use std::rc::Rc;

use speedy2d::dimen::Vector2;
use speedy2d::font::{Font as S2DFont, FormattedTextBlock, TextLayout, TextOptions};

const MIN_FONT_SIZE: u32 = 4;
const MAX_FONT_SIZE: u32 = 64;
const DEFAULT_FONT_SIZE: u32 = 16;

#[derive(Debug, Clone)]
pub struct Font {
    pub name: String,
    pub size: u32,
    pub char_width: f32,
    pub char_height: f32,
    pub editor_size: Vector2<f32>,
    pub style_changed: bool,
    pub s2d_font: S2DFont,
}

impl Font {
    pub fn new(bytes: &[u8], editor_width: f32, editor_height: f32) -> Self {
        let font_file_content = bytes;
        let s2d_font = S2DFont::new(font_file_content).unwrap();
        let font_layout = s2d_font.layout_text("a", 2.0 * DEFAULT_FONT_SIZE as f32, TextOptions::default());
        Self {
            name: "src".to_string(),
            size: DEFAULT_FONT_SIZE,
            char_width: font_layout.width(),
            char_height: font_layout.height(),
            editor_size: (editor_width, editor_height).into(),
            style_changed: false,
            s2d_font,
        }
    }

    pub fn change_font_size(&mut self, amount: i32) {
        self.size = (self.size as i32 + amount).clamp(MIN_FONT_SIZE as i32, MAX_FONT_SIZE as i32) as u32;
        let font_layout = self.s2d_font.layout_text("a", 2.0 * self.size as f32, TextOptions::default());
        self.char_width = font_layout.width();
        self.char_height = font_layout.height();
        self.style_changed = true;
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

    pub fn layout_text(&self, text: &str, text_layout_options: TextOptions) -> Rc<FormattedTextBlock> {
        let escaped_text = self.format(text)
            .replace('\t', "  ")// Just for rendering
            .replace(" " ,"\u{a0}");  // Just for rendering
        self.s2d_font.layout_text(&escaped_text, 2.0 * self.size as f32, text_layout_options)
    }

    pub fn on_resize(&mut self, size: Vector2<u32>) {
        self.editor_size = Vector2::new(
            size.x as f32,
            size.y as f32,
        );
    }
}
