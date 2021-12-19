use std::fs;
use std::rc::Rc;
use speedy2d::dimen::Vector2;
use speedy2d::font::{Font as S2DFont, FormattedTextBlock, TextAlignment, TextLayout, TextOptions};

const FONT_SIZE: u32 = 16;
const EDITOR_PADDING: f32 = 5.0;

#[derive(Debug, Clone)]
pub(crate) struct Font {
    pub name: String,
    pub char_width: f32,
    pub char_height: f32,
    pub editor_size: Vector2<f32>,
    pub wrap_index: u32,
    pub s2d_font: S2DFont
}

impl Font {
    pub fn new(src: &str, editor_width: f32, editor_height: f32) -> Self {
        let font_file_content = fs::read(src).unwrap();
        let s2d_font = S2DFont::new(&font_file_content).unwrap();
        let font_layout = s2d_font.layout_text("a", 2.0*FONT_SIZE as f32, TextOptions::default());
        Self {
            name: src.to_string(),
            char_width: font_layout.width(),
            char_height: font_layout.height(),
            editor_size: (editor_width, editor_height).into(), // arbitrary
            wrap_index: (editor_width / font_layout.width()) as u32,
            s2d_font
        }
    }

    pub fn layout_text(&self, text: &str) -> Rc<FormattedTextBlock> {
        let text_layout_options = TextOptions::default()
            .with_wrap_to_width(self.editor_size.x - 2. * EDITOR_PADDING, TextAlignment::Left);
        self.s2d_font.layout_text(text, 2.0*FONT_SIZE as f32, text_layout_options)
    }

    pub fn on_resize(&mut self, size: Vector2<u32>) {
        self.editor_size = Vector2 { x: size.x as f32, y: size.y as f32 };
        self.wrap_index = (self.editor_size.x / self.char_width) as u32;
    }

}
