use std::{cmp, fs};

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use speedy2d::color::Color;

use speedy2d::dimen::Vector2;
use speedy2d::Graphics2D;
use speedy2d::shape::Rectangle;
use speedy2d::window::UserEventSender;

use crate::cursor::Cursor;
use crate::camera::Camera;
use crate::EditorEvent;
use crate::font::Font;
use crate::line::Line;
use crate::range::Range;

const EDITOR_PADDING: f32 = 5.;

pub fn clamp<T: Ord>(min: T, x: T, max: T) -> T {
    return cmp::max(min, cmp::min(x, max));
}

pub(crate) struct Editor {
    pub lines: Vec<Line>,
    pub cursor: Cursor,
    pub camera: Camera,
    pub font: Rc<RefCell<Font>>,
    filepath: Option<String>,
    pub event_sender: Option<UserEventSender<EditorEvent>>,
    pub selection: Range,
}

impl Editor {
    pub fn new(width: f32, height: f32) -> Self {
        let font = Rc::new(RefCell::new(Font::new(
            "resources/font/CourierRegular.ttf",
            width,
            height,
        )));
        Self {
            cursor: Cursor::new(0, 0, Rc::clone(&font)),
            camera: Camera::new(width, height),
            lines: vec![Line::new(Rc::clone(&font))],
            font,
            filepath: Option::None,
            event_sender:Option::None,
            selection: Range::default()
        }
    }

    pub fn set_animation_event_sender(&mut self, aes: Option<UserEventSender<EditorEvent>>) {
        self.cursor.event_sender = aes.clone();
        self.camera.event_sender = aes;
    }

    pub fn on_resize(&mut self, size: Vector2<u32>) { self.font.borrow_mut().on_resize(size) }

    fn get_valid_cursor_position(&mut self, position: Vector2<u32>) -> Vector2<u32> {
        let max_y = self.lines.len() as u32 - 1;
        let y = cmp::min(position.y, max_y);
        let line_length = self.lines[y as usize].buffer.len() as u32;
        let x =  cmp::min(position.x, line_length);
        Vector2::new(x, y)
    }

    pub fn move_cursor(&mut self, position: Vector2<u32>) {
        assert!(self.lines.len() > 0);
        let pos = self.get_valid_cursor_position(position);
        self.cursor.move_to(pos.x, pos.y);
    }

    pub fn move_cursor_relative(&mut self, rel_x: i32, rel_y: i32) {
        let max_y = self.lines.len() as u32 - 1;
        let new_x = (self.cursor.x as i32 + rel_x) as i32;
        let new_y = clamp(0, self.cursor.y as i32 + rel_y, max_y as i32) as u32;

        if new_x < 0 {
            // Go to line before
            if self.cursor.y == 0 { return; }
            let previous_line_buffer_size = self.lines[self.cursor.y as usize - 1].buffer.len() as u32;
            self.cursor.move_to(previous_line_buffer_size, self.cursor.y - 1);
        } else if new_x as usize > self.get_current_buffer().len() {
            // Go to line after
            if self.cursor.y as usize >= self.lines.len() - 1 {return; }
            self.cursor.move_to(0, self.cursor.y + 1);
        } else {
            // Classic move inside a line
            // Check if x if inside new_y buffer limits
            let new_buffer_len = self.lines[new_y as usize].buffer.len() as i32;
            if new_x >= new_buffer_len {
                self.cursor.move_to(new_buffer_len as u32, new_y);
            } else {
                self.cursor.move_to(new_x as u32, new_y);
            }
        }
        if self.cursor.computed_y() < self.camera.computed_y() + self.camera.safe_zone_size {
            self.camera.move_y(self.cursor.computed_y() - self.camera.computed_y() - self.camera.safe_zone_size)
        } else if self.cursor.computed_y() - self.camera.computed_y() > self.camera.height - self.camera.safe_zone_size {
            self.camera.move_y(self.cursor.computed_y() - self.camera.computed_y() - self.camera.height + self.camera.safe_zone_size)
        }
    }

    fn get_current_line(&mut self) -> &mut Line {
        &mut self.lines[self.cursor.y as usize]
    }

    fn get_current_buffer(&mut self) -> &mut Vec<String> {
        &mut self.get_current_line().buffer
    }

    fn delete_selection(&mut self) {
        if self.selection.is_valid() {
            let initial_i = cmp::min(self.selection.start.unwrap().y, self.selection.end.unwrap().y) as usize;
            let lines_indices = self.selection.get_lines_index(&self.lines);
            for (i, indices) in lines_indices.iter().enumerate() {
                let start = cmp::min(indices.0, indices.1) as usize;
                let end = cmp::max(indices.0, indices.1) as usize;
                &self.lines[initial_i + i].buffer.drain(start .. end);
            }
            for i in 0 .. lines_indices.len() {
                let index = initial_i + lines_indices.len() - i - 1;
                if self.lines[index].buffer.len() == 0 {
                    self.lines.remove(index);
                }
            }
            let selection_start = self.selection.get_real_start().unwrap();
            self.move_cursor(Vector2::new(selection_start.x, selection_start.y));
            self.selection.reset();
        }
    }

    pub fn add_char(&mut self, c: String) {
        self.delete_selection();
        let pos = self.cursor.x;
        self.get_current_buffer().insert(pos as usize, c);
        self.move_cursor_relative(1, 0);
    }

    pub fn delete_char(&mut self) {
        if self.selection.is_valid() {
            self.delete_selection();
            return;
        }
        let pos = self.cursor.x as i32;
        let row = self.cursor.y;
        if pos == 0 {
            if row == 0 { return; } // The first line should never be deleted
            let buffer = self.get_current_buffer().clone();
            let previous_buffer = &mut self.lines[row as usize - 1].buffer;
            let previous_line_buffer_previous_size = previous_buffer.len() as u32;
            buffer.iter().for_each(|c| previous_buffer.push(c.clone()));
            self.cursor.move_to(previous_line_buffer_previous_size, self.cursor.y - 1);
            self.lines.remove(row as usize);
        } else {
            let buffer = self.get_current_buffer();
            assert!(pos <= buffer.len() as i32);
            buffer.remove(pos as usize - 1);
            self.move_cursor_relative(-1, 0);
        }
    }

    pub fn new_line(&mut self) {
        let new_line = Line::new(Rc::clone(&self.font));
        let index = self.cursor.y as usize + 1;
        self.lines.insert(index, new_line);
        // Pattern matching for new line
        if index == 1 {
            self.cursor.move_to(0, self.cursor.y + 1);
            return;
        }
        let line_before_buffer= self.lines.get(index - 1).unwrap().buffer.clone();
        let last_line = self.lines.get_mut(index).unwrap();
        let text = line_before_buffer.join("");
        let nb_whitespace = text.len() - text.trim_start().len();
        if text.trim_start().starts_with('-') {
            let new_text = " ".repeat(nb_whitespace) + "- ";
            last_line.add_text(&new_text);
            self.cursor.move_to(nb_whitespace as u32 + 2, self.cursor.y + 1);
        } else {
            self.cursor.move_to(0, self.cursor.y + 1);
        }
    }

    pub fn shortcut(&mut self, c: char) {
        match c {
            's' => self.save_to_file(),
            'o' => self.load_file("output.txt"),
            'w' | 'q' => std::process::exit(0),
            _ => {}
        }
    }

    pub fn get_mouse_position_index(&mut self, position: Vector2<f32>) -> Vector2<u32> {
        let position = Vector2::new(
            (position.x as f32 / self.font.borrow().char_width) as u32,
            (position.y as f32 / self.font.borrow().char_height) as u32,
        );
        self.get_valid_cursor_position(position)
    }

    pub fn begin_selection(&mut self) {
        self.selection.start((self.cursor.x, self.cursor.y).into());
    }

    pub fn update_selection(&mut self, position: Vector2<f32>) {
        let mouse_position = self.get_mouse_position_index(position);
        if self.selection.start.is_none() {
            self.selection.start(mouse_position);
        }
        self.selection.end(mouse_position);
    }

    fn get_save_path(&self) -> Option<String> {
        Some("output.txt".to_string())
    }

    pub fn save_to_file(&mut self) {
        let filename = if let Some(f) = self.filepath.clone() { f } else if let Some(f) = self.get_save_path() { f } else { String::new() };
        if filename.len() == 0 { return; }
        if let Some(filepath) = &self.filepath {
            if *filepath != filename { self.filepath = Some(filename.clone()); }
        }
        let mut data = String::new();
        for (i, line) in (&self.lines).iter().enumerate() {
            data.push_str(&line.buffer.clone().join(""));
            if i + 1 != self.lines.len() { data.push('\n') }
        }
        fs::write(&filename, &data).expect(&format!("Unable to write file to {}", filename));

    }

    pub fn load_file(&mut self, filename: &str) {
        self.lines = vec![Line::new(Rc::clone(&self.font))];
        self.filepath = Some(filename.to_string());
        let file_content = fs::read_to_string(&filename).expect(&format!("Unable to load file to {}", filename));
        for (i, line) in file_content.split('\n').enumerate() {
            if i < self.lines.len() {
                // Push a line rather than using the self.new_line method in order to avoid patern matching
                self.lines.push(Line::new(Rc::clone(&self.font)));
            }
            self.lines[i].add_text(line);
        }
        self.cursor.move_to(0, 0);
        self.update_text_layout();
    }

    pub fn update_text_layout(&mut self) {
        for line in &mut self.lines {
            line.update_text_layout();
        }
        self.move_cursor(Vector2::new(self.cursor.x, self.cursor.y)); // Hack to get the cursor to a valid position
    }

    pub fn update(&mut self, dt: f32) {
        if let Some(animation_x) = &mut self.cursor.animation.x {
            if !animation_x.has_started {
                animation_x.start();
            }
            animation_x.update(dt);
            if animation_x.is_ended {
                self.cursor.animation.x = Option::None;
            }
        }
        if let Some(animation_y) = &mut self.cursor.animation.y {
            if !animation_y.has_started {
                animation_y.start();
            }
            animation_y.update(dt);
            if animation_y.is_ended {
                self.cursor.animation.y = Option::None;
            }
        }
        if let Some(animation_x) = &mut self.camera.animation.x {
            if !animation_x.has_started {
                animation_x.start();
            }
            animation_x.update(dt);
            if animation_x.is_ended {
                self.camera.animation.x = Option::None;
            }
        }
        if let Some(animation_y) = &mut self.camera.animation.y {
            if !animation_y.has_started {
                animation_y.start();
            }
            animation_y.update(dt);
            if animation_y.is_ended {
                self.camera.animation.y = Option::None;
            }
        }
    }

    pub fn render(&mut self, graphics: &mut Graphics2D) {
        let mut previous_line_height = 0.;
        for (i, line) in self.lines.iter().enumerate() {
            line.render(
                - self.camera.computed_x() + EDITOR_PADDING,
                - self.camera.computed_y() + EDITOR_PADDING + previous_line_height * (i as f32),
                graphics,
            );
            previous_line_height = if line.formatted_text_block.height() > 0. {
                line.formatted_text_block.height()
            } else {
                line.font.borrow().char_height
            };
        }
        self.cursor.render(&self.camera, graphics);
        self.camera.render(graphics);
        self.selection.render(Rc::clone(&self.font), &self.lines, graphics);
    }
}
