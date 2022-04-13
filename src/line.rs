use std::rc::Rc;
use std::cell::RefCell;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::font::{FormattedTextBlock, TextAlignment, TextOptions};
use speedy2d::Graphics2D;
use crate::style_range::StyleRange;

use crate::font::Font;
use crate::range::Range;
use crate::range_trait::RangeTrait;
use crate::render_helper::draw_rectangle;

const INITIAL_LINE_CAPACITY: usize = 1024;

#[derive(Derivative)]
#[derivative(Clone)]
pub struct StyleBlock {
    formatted_text_block: Rc<FormattedTextBlock>,
    offset: f32,
    color: Color,
}

impl StyleBlock {
    pub fn new_unstyle(ftb: Rc<FormattedTextBlock>) -> Self {
        Self {
            formatted_text_block: ftb,
            offset: 0.0,
            color: Color::BLACK
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug, Clone)]
pub struct Line {
    pub buffer: Vec<String>,
    pub font: Rc<RefCell<Font>>,
    pub alignment: TextAlignment,
    pub alignment_offset: f32,
    #[derivative(Debug = "ignore")]
    pub style_block: Vec<StyleBlock>,
    previous_string: String,
}

impl Line {
    pub fn new(font: Rc<RefCell<Font>>) -> Self {
        let style_block = vec![StyleBlock::new_unstyle(font.borrow().layout_text("", TextOptions::default()))];
        Line {
            buffer: Vec::with_capacity(INITIAL_LINE_CAPACITY),
            previous_string: String::new(),
            alignment: TextAlignment::Left,
            alignment_offset: 0.,
            style_block,
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
            TextAlignment::Center => (editor_width - self.get_unstyled_ftb().width()) / 2.,
            TextAlignment::Right => editor_width - self.get_unstyled_ftb().width()
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
        let char_jump_list = [' ', '_', '-', '/', '(', ')', '[', ']', '{', '}', '"', '\''];
        let chars: Vec<char> =  self.buffer.join("").chars().collect();
        let max_indices = chars.len() as u32;
        while start_index > 0 &&  char_jump_list.contains(&chars[start_index as usize - 1]) { start_index -= 1 }
        while end_index < max_indices &&  char_jump_list.contains(&chars[end_index as usize]) { end_index += 1 }
        while start_index > 0 && !char_jump_list.contains(&chars[start_index as usize - 1]) { start_index -= 1; }
        while end_index < max_indices && !char_jump_list.contains(&chars[end_index as usize]) { end_index += 1; }
        (start_index, end_index)
    }

    pub fn get_unstyled_ftb(&self) -> &Rc<FormattedTextBlock> {
        &self.style_block[0].formatted_text_block
    }

    /// return the difference of length between the raw buffer and the styled text
    pub fn update_text_layout(&mut self, y: usize, style_buffer: &Vec<StyleRange>) -> i32 {
        let string = self.get_text();
        let mut font = self.font.borrow();
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
        if font_formatted_string != self.previous_string || font.style_changed {
            // The first element is the all line without style
            self.style_block = vec![StyleBlock::new_unstyle(font.layout_text(&font_formatted_string, TextOptions::default()))];
            let line_range = Range::new((0, y as u32).into(), (self.buffer.len() as u32, y as u32).into());
            let line_style_buffer: Vec<&StyleRange> = style_buffer
                .iter()
                .filter(|sr| line_range.include(&sr.range) || sr.range.include(&line_range))
                .collect();

            for style_range in line_style_buffer.iter() {
                let start = if style_range.get_real_start().unwrap().y == y as u32 { style_range.get_real_start().unwrap().x as usize } else { 0 };
                let end = if style_range.get_real_end().unwrap().y == y as u32 { style_range.get_real_end().unwrap().x as usize } else { self.buffer.len() };
                let ftb =
                    if style_range.bold { font.get_bold().layout_text(&font_formatted_string[start .. end],  TextOptions::default())}
                    else { font.layout_text(&font_formatted_string[start .. end],  TextOptions::default())};
                self.style_block.push(StyleBlock {
                    formatted_text_block: ftb,
                    offset: start as f32 * font.char_width,
                    color: style_range.color,
                });
            }
            self.previous_string = font_formatted_string;
        }
        diff
    }

    pub fn render(&self, x: f32, y: f32, graphics: &mut Graphics2D) {
        for sb in &self.style_block {
            let x = x + self.alignment_offset + sb.offset;
            let ftb = &sb.formatted_text_block;
            // draw_rectangle(x, y, ftb.width(), ftb.height(), Color::WHITE, graphics);
            graphics.draw_text(Vector2::new(x, y), sb.color, ftb);
        }
    }
}
