use std::rc::Rc;
use std::cell::RefCell;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::font::FormattedTextBlock;
use speedy2d::Graphics2D;

use crate::font::Font;

const INITIAL_LINE_CAPACITY: usize = 1024;

#[derive(Derivative)]
#[derivative(Debug, Clone)]
pub(crate) struct Line {
    pub buffer: Vec<String>,
    pub font: Rc<RefCell<Font>>,
    pub wrap_index: u32,
    pub wrap_y: u32, // Number of displayed lines caused by the wrap behaviour 
    #[derivative(Debug = "ignore")]
    pub formatted_text_block: Rc<FormattedTextBlock>,
    previous_string: String,
}

impl Line {
    pub fn new(font: Rc<RefCell<Font>>) -> Self {
        let formatted_text_block = font.borrow().layout_text("");
        let wrap_index = (font.borrow().editor_size.y / font.borrow().char_width) as u32;
        Line {
            buffer: Vec::with_capacity(INITIAL_LINE_CAPACITY),
            previous_string: String::new(),
            formatted_text_block,
            font,
            wrap_index,
            wrap_y: 0,
        }
    }

    pub fn add_text(&mut self, text: &str) {
        for c in text.chars() {
            self.buffer.push(c.to_string());
        }
    }

    pub fn get_word_at(&self, index: u32) -> (u32, u32) {
        let mut start_index = index;
        let mut end_index = index;
        let line_str = self.buffer.join("");
        let chars: Vec<char> = line_str.chars().collect();
        while start_index > 0 && chars[start_index as usize - 1] != ' ' {
            start_index -= 1;
        }
        while end_index < line_str.len() as u32 && chars[end_index as usize] != ' ' {
            end_index += 1;
        }
        (start_index, end_index)
    }

    pub fn update_text_layout(&mut self) -> i32 { // return the difference of length
        let string = self.buffer.clone().join("");
        let font_formated_string = self.font.borrow().format(&string);
        let mut diff: i32 = 0;
        if string != font_formated_string {
            diff = string.len() as i32 - font_formated_string.len() as i32;
            self.buffer = font_formated_string
                .split("")
                .map(|c| c.to_string())
                .filter(|s| s != "")
                .collect();
        }
        if font_formated_string != self.previous_string {
            self.formatted_text_block = self.font.borrow().layout_text(&font_formated_string);
            self.previous_string = font_formated_string;
        }
        diff
    }

    pub fn render(&self, x: f32, y: f32, graphics: &mut Graphics2D) {
        graphics.draw_text(Vector2::new(x, y), Color::BLACK, &self.formatted_text_block);
    }
}
