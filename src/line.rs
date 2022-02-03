use std::rc::Rc;
use std::cell::RefCell;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::font::{FormattedTextBlock, TextAlignment, TextOptions};
use speedy2d::Graphics2D;

use crate::font::Font;

const INITIAL_LINE_CAPACITY: usize = 1024;

#[derive(Derivative)]
#[derivative(Debug, Clone)]
pub struct Line {
    pub buffer: Vec<String>,
    pub font: Rc<RefCell<Font>>,
    pub alignment: TextAlignment,
    pub alignment_offset: f32,
    #[derivative(Debug = "ignore")]
    pub formatted_text_block: Rc<FormattedTextBlock>,
    previous_string: String,
}

impl Line {
    pub fn new(font: Rc<RefCell<Font>>) -> Self {
        let formatted_text_block = font.borrow().layout_text("", TextOptions::default());
        Line {
            buffer: Vec::with_capacity(INITIAL_LINE_CAPACITY),
            previous_string: String::new(),
            alignment: TextAlignment::Left,
            alignment_offset: 0.,
            formatted_text_block,
            font,
        }
    }

    pub fn add_text(&mut self, text: &str) {
        for c in text.chars() {
            self.buffer.push(c.to_string());
        }
    }

    /// Empty the mline buffer
    pub fn empty(&mut self) {
        let length = self.buffer.len();
        self.buffer.drain(0 .. length);
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.join("") == ""
    }

    pub fn set_alignment(&mut self, alignment: TextAlignment) {
        let editor_width = self.font.borrow().editor_size.x;
        self.alignment_offset = match alignment {
            TextAlignment::Left => 0.,
            TextAlignment::Center => (editor_width - self.formatted_text_block.width()) / 2.,
            TextAlignment::Right => editor_width - self.formatted_text_block.width()
        };
        self.alignment = alignment;
    }

    pub fn get_text(&self) -> String {
        self.buffer.join("")
    }

    pub fn get_word_count(&self) -> u32 { self.buffer.join("").split(' ').filter(|w| *w != "").count() as u32 }

    pub fn get_word_at(&self, index: u32) -> (u32, u32) {
        let mut start_index = index;
        let mut end_index = index;
        let chars: Vec<char> =  self.buffer.join("").chars().collect();
        while start_index > 0 && chars[start_index as usize - 1] != ' ' {
            start_index -= 1;
        }
        while end_index < chars.len() as u32 && chars[end_index as usize] != ' ' {
            end_index += 1;
        }
        (start_index, end_index)
    }

    pub fn get_next_jump(&self, index: u32) -> (u32, u32) {
        let mut start_index = index;
        let mut end_index = index;
        let char_jump_list = [' ', '_', '-', '/'];
        let chars: Vec<char> =  self.buffer.join("").chars().collect();
        let max_indices = chars.len() as u32;
        while start_index > 0 &&  char_jump_list.contains(&chars[start_index as usize - 1]) { start_index -= 1 }
        while end_index < max_indices &&  char_jump_list.contains(&chars[end_index as usize]) { end_index += 1 }
        while start_index > 0 && !char_jump_list.contains(&chars[start_index as usize - 1]) { start_index -= 1; }
        while end_index < max_indices && !char_jump_list.contains(&chars[end_index as usize]) { end_index += 1; }
        (start_index, end_index)
    }

    pub fn update_text_layout(&mut self) -> i32 { // return the difference of length
        let string = self.get_text();
        let font = self.font.borrow();
        let font_formatted_string = font.format(&string);
        let mut diff: i32 = 0;
        if string != font_formatted_string {
            diff = self.buffer.len() as i32;
            self.buffer = font_formatted_string
                .split("")
                .map(|c| c.to_string())
                .filter(|s| s != "")
                .collect();
            diff -= self.buffer.len() as i32;
        }
        if font_formatted_string != self.previous_string || font.style_changed{
            // self.formatted_text_block = font.layout_text(&font_formatted_string, TextOptions::default().with_wrap_to_width(font.editor_size.x, self.alignment.clone()));
            self.formatted_text_block = font.layout_text(&font_formatted_string, TextOptions::default());
            self.previous_string = font_formatted_string;
        }
        diff
    }

    pub fn render(&self, x: f32, y: f32, graphics: &mut Graphics2D) {
        graphics.draw_text(Vector2::new(x + self.alignment_offset, y), Color::BLACK, &self.formatted_text_block);
    }
}
