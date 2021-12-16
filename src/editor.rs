use std::rc::Rc;
use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::font::{FormattedTextBlock};
use speedy2d::Graphics2D;
use crate::cursor::Cursor;
use crate::font::Font;

const FONT_SIZE: u32 = 16;
const EDITOR_PADDING: u32 = 5;

pub(crate) struct Editor {
    pub buffer: Vec<String>,
    pub cursor: Cursor,
    pub font: Font,
    previous_string: String,
    formatted_text_block: Rc<FormattedTextBlock>
}

impl Editor {
    pub fn new() -> Editor {
        let font = Font::new("resources/font/CourierRegular.ttf");
        Editor {
            buffer: Vec::with_capacity(2048),
            cursor: Cursor { x: 0, y: 0, font: font.clone() },
            formatted_text_block: font.layout_text(""),
            previous_string: String::new(),
            font,
        }
    }

    pub fn move_cursor_relative(&mut self, rel_x: i32, rel_y: i32) {
        if self.cursor.x as i32 + rel_x <= self.buffer.len() as i32 {
            self.cursor.move_relative(rel_x, rel_y);
        }
    }

    pub fn add_char(&mut self, c: String) {
        self.buffer.insert(self.cursor.x as usize, c);
        self.move_cursor_relative(1, 0);
    }

    pub fn delete_char(&mut self) {
        if self.buffer.len() == 0 { return };
        self.buffer.remove(self.cursor.x as usize - 1);
        self.move_cursor_relative(-1, 0);
    }

    pub fn update(&mut self) {
        let string = self.buffer.clone().join("");
        if string != self.previous_string {
            self.formatted_text_block = self.font.layout_text(&string);
            self.previous_string = string;
        }
    }

    pub fn render(&mut self, graphics: &mut Graphics2D) {
        graphics.draw_text(Vector2::new(EDITOR_PADDING as f32, EDITOR_PADDING as f32), Color::BLACK, &self.formatted_text_block);
        self.cursor.render(graphics);
    }
}
