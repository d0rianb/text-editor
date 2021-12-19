use std::cell::RefCell;
use std::rc::Rc;
use speedy2d::dimen::Vector2;
use speedy2d::Graphics2D;

use crate::line::Line;
use crate::cursor::Cursor;
use crate::font::Font;

const FONT_SIZE: f32 = 16.;
const EDITOR_PADDING: f32 = 5.;

pub(crate) struct Editor {
    pub lines: Vec<Line>,
    pub cursor: Cursor,
    pub font: Rc<RefCell<Font>>,
}

impl Editor {
    pub fn new(width: f32, height: f32) -> Self {
        let font = Rc::new(RefCell::new(Font::new("resources/font/CourierRegular.ttf", width, height)));
        Editor {
            cursor: Cursor { x: 0, y: 0, font: Rc::clone(&font) },
            lines: vec![Line::new(Rc::clone(&font))],
            font,
        }
    }

    pub fn on_resize(&mut self, size: Vector2<u32>) {
       self.font.borrow_mut().on_resize(size);
    }

    pub fn move_cursor_relative(&mut self, rel_x: i32, rel_y: i32) {
        // todo!("refactor");
        let new_x = (self.cursor.x as i32 + rel_x) as u32;
        if self.cursor.x as i32 + rel_x < 0i32 && self.cursor.y > 0 {
            let previous_line_buffer_size = self.lines[self.cursor.y as usize - 1].buffer.len() as u32;
            self.cursor.move_to(previous_line_buffer_size, self.cursor.y - 1);
        }
        if new_x as usize <= self.get_current_buffer().len() {
            self.cursor.move_relative(rel_x, rel_y);
        }
    }

    fn get_current_line(&mut self) -> &mut Line {
        &mut self.lines[self.cursor.y as usize]
    }

    fn get_current_buffer(&mut self) -> &mut Vec<String> {
        &mut self.get_current_line().buffer
    }

    pub fn add_char(&mut self, c: String) {
        let pos = self.get_current_line().wrap_y * self.font.borrow().wrap_index + self.cursor.x;
        self.get_current_buffer().insert(pos as usize, c);
        self.move_cursor_relative(1, 0);
    }

    pub fn delete_char(&mut self) {
        let pos = self.cursor.x as i32;
        let buffer = self.get_current_buffer();
        if pos == 0 {
            if self.cursor.y == 0 { return; } // The first line should never be deleted
            assert!(self.cursor.y < self.lines.len() as u32);
            self.lines.remove(self.cursor.y as usize);
            if self.cursor.y > 0 {
                let previous_line_buffer_size = self.lines[self.cursor.y as usize - 1].buffer.len() as u32;
                self.cursor.move_to(previous_line_buffer_size, self.cursor.y - 1);
            }
        } else {
            assert!(pos <= buffer.len() as i32);
            buffer.remove(pos as usize - 1);
            self.move_cursor_relative(-1, 0);
        }
    }

    pub fn new_line(&mut self) {
        self.lines.push(Line::new(Rc::clone(&self.font)));
        self.cursor.move_to(0, self.cursor.y + 1);
    }

    pub fn update_text_layout(&mut self) {
        for line in &mut self.lines {
            line.update_text_layout();
        }
    }

    pub fn render(&mut self, graphics: &mut Graphics2D) {
        let mut previous_line_height = 0.;
        for (i, line) in self.lines.iter().enumerate() {
            line.render(EDITOR_PADDING, EDITOR_PADDING + previous_line_height * (i as f32), graphics);
            previous_line_height = if line.formatted_text_block.height() > 0. { line.formatted_text_block.height() } else { line.font.borrow().char_height };
        }
        self.cursor.render(graphics);
    }
}
