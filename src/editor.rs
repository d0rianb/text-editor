use std::{cmp, fs};

use std::cell::RefCell;
use std::rc::Rc;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::font::TextAlignment;
use speedy2d::Graphics2D;
use speedy2d::shape::Rectangle;
use speedy2d::window::{ModifiersState, UserEventSender};

use clipboard::ClipboardProvider;
use clipboard::ClipboardContext;
use lazy_static::lazy_static;
use regex::Regex;

extern crate yaml_rust;
use yaml_rust::{Yaml, YamlLoader};

use crate::cursor::Cursor;
use crate::camera::Camera;
use crate::contextual_menu::{ContextualMenu, MenuItem};
use crate::{EditorEvent, MenuAction};
use crate::font::Font;
use crate::line::Line;
use crate::range::{Range};

pub const EDITOR_PADDING: f32 = 10.;
pub const EDITOR_OFFSET_TOP: f32 = 55.;

pub fn clamp<T: Ord>(min: T, x: T, max: T) -> T {
    return cmp::max(min, cmp::min(x, max));
}

pub struct Editor {
    pub lines: Vec<Line>,
    pub cursor: Cursor,
    pub camera: Camera,
    pub font: Rc<RefCell<Font>>,
    pub modifiers : ModifiersState,
    pub filepath: Option<String>,
    pub event_sender: Option<UserEventSender<EditorEvent>>,
    pub selection: Range,
    pub copy_buffer: Vec<String>,
    pub underline_buffer: Vec<Range>,
    pub bold_buffer: Vec<Range>,
    pub menu: ContextualMenu,
}

impl Editor {
    pub fn new(width: f32, height: f32) -> Self {
        let font = Rc::new(RefCell::new(Font::new(
            include_bytes!("../resources/font/CourierRegular.ttf"),
            // "resources/font/Monaco.ttf",
            width - EDITOR_PADDING*2.,
            height - EDITOR_OFFSET_TOP - EDITOR_PADDING*2.,
        )));
        let system_font = Rc::new(Font::new(include_bytes!("../resources/font/Roboto-Regular.ttf"), width, height));
        Self {
            cursor: Cursor::new(0, 0, Rc::clone(&font)),
            camera: Camera::new(width - EDITOR_PADDING*2., height - EDITOR_OFFSET_TOP - EDITOR_PADDING*2.),
            lines: vec![Line::new(Rc::clone(&font))],
            font,
            modifiers: ModifiersState::default(),
            filepath: Option::None,
            event_sender: Option::None,
            selection: Range::default(),
            copy_buffer: vec![],
            underline_buffer: vec![],
            bold_buffer: vec![],
            menu: ContextualMenu::new(system_font),
        }
    }

    pub fn set_animation_event_sender(&mut self, es: Option<UserEventSender<EditorEvent>>) {
        self.cursor.event_sender = es.clone();
        self.camera.event_sender = es.clone();
        self.menu.event_sender = es.clone();
    }

    pub fn on_resize(&mut self, size: Vector2<u32>) { self.font.borrow_mut().on_resize(size) }

    fn get_valid_cursor_position(&mut self, position: Vector2<u32>) -> Vector2<u32> {
        let max_y = self.lines.len() as u32 - 1;
        let y = cmp::min(position.y, max_y);
        let line = &self.lines[y as usize];
        let x =  (position.x as i32 - (line.alignement_offset / self.font.borrow().char_width + 0.5) as i32).clamp(0,  line.buffer.len() as i32) as u32;
        Vector2::new(x, y)
    }

    pub fn move_cursor(&mut self, position: Vector2<u32>) {
        assert!(self.lines.len() > 0);
        let pos = self.get_valid_cursor_position(position);
        if pos.x != self.cursor.x || pos.y != self.cursor.y {
            self.cursor.move_to(pos.x, pos.y);
        }
    }

    pub fn move_cursor_relative(&mut self, rel_x: i32, rel_y: i32) {
        let max_y = self.lines.len() as u32 - 1;
        let mut new_x = (self.cursor.x as i32 + rel_x) as i32;
        let mut new_y = clamp(0, self.cursor.y as i32 + rel_y, max_y as i32);

        if self.modifiers.shift() && self.selection.start.is_none() {
            self.selection.start(Vector2::new(self.cursor.x, self.cursor.y));
        }

        if self.modifiers.alt() {
            // Move to the next word
            let (start, end) = self.lines[self.cursor.y as usize].get_word_at(self.cursor.x);
            if rel_x < 0 && start != self.cursor.x  {
                new_x = start as i32;
            } else if rel_x > 0 && end != self.cursor.x  {
                new_x = end as i32;
            }
        } else if self.modifiers.logo() {
            if rel_x < 0  {
                new_x = 0;
            } else if rel_x > 0 {
                new_x = self.lines[self.cursor.y as usize].buffer.len() as i32;
            }
            if rel_y < 0 {
                new_y = 0;
            } else if rel_y > 0 {
                new_y = self.lines.len() as i32 - 1;
            }
        }

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
                self.cursor.move_to(new_buffer_len as u32, new_y as u32);
            } else {
                self.cursor.move_to(new_x as u32, new_y as u32);
            }
        }
        // Update selection
        if self.modifiers.shift() {
            self.selection.end(Vector2::new(self.cursor.x, self.cursor.y));
        } else if (rel_x.abs() > 0 || rel_y.abs() > 0) && self.selection.is_valid() {
            self.selection.reset();
        }
        self.update_camera();
    }

    fn update_camera(&mut self) {
        // Vertical Scroll
        if self.camera.get_cursor_real_y(&self.cursor) < self.camera.computed_y() + self.camera.safe_zone_size {
            self.camera.move_y(self.camera.get_cursor_real_y(&self.cursor) - self.camera.computed_y() - self.camera.safe_zone_size)
        } else if EDITOR_PADDING + self.cursor.computed_y() - self.camera.computed_y() > self.camera.height - self.camera.safe_zone_size {
            self.camera.move_y(EDITOR_PADDING + self.cursor.computed_y() - self.camera.computed_y() - self.camera.height + self.camera.safe_zone_size)
        }
    }

    fn get_current_line(&mut self) -> &mut Line {
        &mut self.lines[self.cursor.y as usize]
    }

    fn get_current_buffer(&mut self) -> &mut Vec<String> {
        &mut self.get_current_line().buffer
    }

    pub fn add_char(&mut self, c: String) {
        if self.modifiers.logo() {
            let chars: Vec<char> = c.chars().collect();
            return self.shortcut(chars[0]);
        }
        // matching template
        let mut after = "";
        for template in [("(", ")"), ("[", "]"), ("{", "}"), ("\"", "\"")] {
            if &c == template.0 { after = template.1; break }
        }
        if after == "" { self.delete_selection(); }
        let mut pos = if self.selection.is_valid() { self.selection.get_real_start().unwrap() } else { Vector2::new(self.cursor.x, self.cursor.y) };
        self.get_current_buffer().insert(pos.x as usize, c);
        if after != "" {
            let after_pos = if self.selection.is_valid() { self.selection.get_real_end().unwrap() } else { Vector2::new(self.cursor.x, self.cursor.y) };
            self.lines[after_pos.y as usize].buffer.insert(after_pos.x as usize + 1, after.into());
        }
        self.move_cursor_relative(1, 0);
        self.selection.reset();
    }

    pub fn delete_char(&mut self) {
        if self.modifiers.alt() || self.modifiers.logo()  {
            self.begin_selection();
            self.move_cursor_relative(-1, 0);
            self.end_selection();
        }
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
            // Auto delete the matching template char if there are next to each other - ex: ""
            if buffer.len() as i32 > pos && buffer.get(pos as usize - 1) == buffer.get(pos as usize) {
                if ["(", "[", "{", "\""].contains(&buffer.get(pos as usize - 1).unwrap().as_str()) {
                    buffer.remove(pos as usize);
                }
            }
            buffer.remove(pos as usize - 1);
            self.move_cursor_relative(-1, 0);
        }
        self.selection.reset();
        self.update_text_layout();
    }

    pub fn new_line(&mut self) {
        self.delete_selection();
        let mut new_line = Line::new(Rc::clone(&self.font));
        let index = self.cursor.y as usize + 1;
        let x = self.cursor.x as usize;
        let current_buffer = self.get_current_buffer();
        let text_after_cursor = current_buffer.drain(x .. current_buffer.len());
        new_line.add_text(&text_after_cursor.as_slice().join(""));
        drop(text_after_cursor);
        self.lines.insert(index, new_line);
        // Pattern matching for new line
        if index == 1 {
            self.cursor.move_to(0, self.cursor.y + 1);
            return;
        }
        let line_before_buffer= self.lines.get(index - 1).unwrap().buffer.clone();
        let line_before_alignement = self.lines.get(index - 1).unwrap().alignement.clone();
        let last_line = self.lines.get_mut(index).unwrap();
        last_line.set_alignement(line_before_alignement); // Preserve the alignement
        let text = line_before_buffer.join("");
        let nb_whitespace = text.len() - text.trim_start().len();
        if text.trim_start().starts_with('-') &&  text.trim().len() > 1 {
            let new_text = " ".repeat(nb_whitespace) + "- ";
            last_line.add_text(&new_text);
            self.cursor.move_to(nb_whitespace as u32 + 2, self.cursor.y + 1);
            self.menu.open_with(vec![MenuItem::new("Annuler", MenuAction::CancelChip) ]);
        } else {
            self.cursor.move_to(0, self.cursor.y + 1);
        }
    }

    pub fn get_mouse_position_index(&mut self, position: Vector2<f32>) -> Vector2<u32> {
        let pos = Vector2::new(
            ((position.x as f32 + self.camera.computed_x()) / self.font.borrow().char_width + 0.5) as u32,
            ((position.y as f32 + self.camera.computed_y()) / self.font.borrow().char_height) as u32,
        );
        self.get_valid_cursor_position(pos)
    }

    pub fn shortcut(&mut self, c: char) {
        match c {
            's' => self.save(),
            'o' => self.load(),
            'u' => self.underline(),
            'c' => self.copy(),
            'v' => self.paste(),
            'x' => { self.copy(); self.delete_selection() },
            'a' => self.select_all(),
            'l' => self.select_current_line(),
            'L' => { self.select_current_line(); self.delete_selection() },
            'w' | 'q' => std::process::exit(0),
            'd' => self.select_current_word(),
            'D' => { self.select_current_word(); self.delete_selection() },
            '+' | '=' => self.increase_font_size(),
            '-' => self.decrease_font_size(),
            'n' => self.menu.open_with(vec![]),
            _ => {}
        }
    }

    pub fn begin_selection(&mut self) {
        self.selection.start((self.cursor.x, self.cursor.y).into());
    }

    pub fn end_selection(&mut self) {
        self.selection.end((self.cursor.x, self.cursor.y).into());
    }

    pub fn update_selection(&mut self, position: Vector2<f32>) {
        let mouse_position = self.get_mouse_position_index(position);
        self.selection.end(mouse_position);
        self.move_cursor(mouse_position);
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
                if self.lines[index].buffer.len() == 0 && self.lines.len() > 1 {
                    self.lines.remove(index);
                }
            }
            let selection_start = self.selection.get_real_start().unwrap();
            self.move_cursor(Vector2::new(selection_start.x, selection_start.y));
            self.selection.reset();
        }
    }

    fn select_current_word(&mut self) {
        let (start, end) = self.lines[self.cursor.y as usize].get_word_at(self.cursor.x);
        self.selection = Range::new(
            Vector2::new(start, self.cursor.y),
            Vector2::new(end, self.cursor.y),
        )
    }

    fn select_all(&mut self) {
        let last_line_length = self.lines.last().unwrap().buffer.len() as u32;
        self.selection.start(Vector2::ZERO);
        self.selection.end(Vector2::new(last_line_length, self.lines.len() as u32 - 1));
    }

    fn select_current_line(&mut self) {
        let line = &self.lines[self.cursor.y as usize];
        let line_selection = Range::new(
            Vector2::new(0, self.cursor.y),
            Vector2::new(line.buffer.len() as u32, self.cursor.y)
        );
        self.selection.add(line_selection);
        self.move_cursor(Vector2::new(0, self.cursor.y + 1))
    }

    /// Add a range to a buffer according to the underline/bold rules
    fn add_range_to_buffer(range: Range, buffer: &mut Vec<Range>) {
        if !range.is_valid() { return; }
        let len = buffer.len();
        for mut i in 0 .. len {
            assert!(len >= 1);
            i = len - 1 - i;
            let buffer_range = buffer.get_mut(i).unwrap();
            if range == *buffer_range { buffer.remove(i); return; }
            else if range.include(buffer_range) { buffer.remove(i); }
            else if buffer_range.include(&range) {
                assert!(buffer_range.is_valid());
                let before = Range::new(buffer_range.get_real_start().unwrap(), range.get_real_start().unwrap());
                let after = Range::new(range.get_real_end().unwrap(), buffer_range.get_real_end().unwrap());
                if before.is_valid() { buffer.push(before);  }
                if after.is_valid() { buffer.push(after);  }
                buffer.remove(i);
                return;
            }
        }
        buffer.push(range);
    }

    fn underline(&mut self) {
        Self::add_range_to_buffer(self.selection.clone(),  &mut self.underline_buffer);
    }

    fn bold(&mut self) {
        Self::add_range_to_buffer(self.selection.clone(),  &mut self.bold_buffer);
    }

    pub fn set_line_alignement(&mut self, alignement: TextAlignment) {
        if self.selection.is_valid() {
            let start = self.selection.get_real_start().unwrap().y as usize;
            let end = self.selection.get_real_end().unwrap().y as usize;
            for (i, line) in self.lines.iter_mut().enumerate() {
                if start <= i && i <= end {
                    line.set_alignement(alignement.clone());
                }
            }
        } else {
            self.get_current_line().set_alignement(alignement);
        }
    }

    fn copy(&mut self) {
        if !self.selection.is_valid() { return; }
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        self.copy_buffer = vec![];
        let lines_index = self.selection.get_lines_index(&self.lines);
        let initial_y = self.selection.get_start_y();
        for (i, (start, end)) in lines_index.iter().enumerate() {
            let y = initial_y as usize + i;
            let mut text = String::new();
            for j in *start .. *end {
                let buffer_text = self.lines[y].buffer.get(j as usize);
                if let Some(bt) = buffer_text { text.push_str(bt); }
            }
            text.push_str("\n");
            self.copy_buffer.push(text);
        }
        ctx.set_contents(self.copy_buffer.join("")).unwrap();
    }

    fn paste(&mut self) {
        if self.selection.is_valid() { self.delete_selection(); }
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        let clipboard_content = ctx.get_contents();
        dbg!(clipboard_content);
        // TODO: match on clipboard content
        if self.copy_buffer.is_empty() { return; }
        let buffer_text = self.copy_buffer.join("");
        let mut lines = buffer_text.split("\n").filter(|c| *c != "");
        let mut text = lines.next();
        while text.is_some() {
            self.lines[self.cursor.y as usize].add_text(text.unwrap());
            self.move_cursor_relative(text.unwrap().len() as i32, 0);
            text = lines.next();
            if text.is_some() {
                self.lines.push(Line::new(Rc::clone(&self.font)));
                self.move_cursor_relative(0, 1);
            }
        }
    }

    fn increase_font_size(&mut self) {
        self.font.borrow_mut().change_font_size(2);
        self.update_text_layout();
        self.update_camera();
        self.event_sender.as_ref().unwrap().send_event(EditorEvent::Redraw).unwrap();
    }

    fn decrease_font_size(&mut self) {
        self.font.borrow_mut().change_font_size(-2);
        self.update_text_layout();
        self.update_camera();
        self.event_sender.as_ref().unwrap().send_event(EditorEvent::Redraw).unwrap();
    }

    fn get_prefs_key(&self, key: &str) -> Yaml {
        lazy_static! {
            static ref PREFS_STR: String = fs::read_to_string("resources/prefs.yaml").expect("Can't find the preference file");
            static ref DOCS: Vec<Yaml> = YamlLoader::load_from_str(&PREFS_STR).expect("Invalid preferences");
            static ref PREFS: &'static Yaml = DOCS.get(0).unwrap();
        }
        PREFS[key].clone()
    }

    fn get_recent_files(&self) -> Vec<(String, String)> {
        lazy_static! { static ref NAME_REGEX: Regex = Regex::new(r"([a-zA-Z0-9_-]+).(\w+)$").unwrap(); }
        let files_yaml = self.get_prefs_key("recent_files");
        let files: Vec<&str> = files_yaml.as_vec().unwrap().iter().map(|f| f.as_str().unwrap()).collect();
        let files_with_names: Vec<(String, String)> = files.iter().map(|f| {
            let file_name: String = NAME_REGEX.captures(*f).unwrap().get(0).unwrap().as_str().to_string();
            (file_name, String::from(*f))
        }).collect();
        files_with_names
    }

    /// Ask for the filepath if there is no one specified else save to the current one
    pub fn save(&mut self) {
        if let Some(f) = self.filepath.clone() {
            self.save_to_file(&f);
        } else {
            let recent_files = self.get_recent_files();
            let mut menu_items = vec![MenuItem::new("Save to ...".into(), MenuAction::Void)];
            for (name, path) in recent_files {
                menu_items.push(MenuItem::new(&name, MenuAction::Save(path)));
            }
            self.menu.open_with(menu_items);
        }
    }

    /// Save to a specific file
    pub fn save_to_file(&mut self, filepath: &str) {
        // TODO: check that the path is valid
        let valid_filepath = fs::canonicalize(filepath).expect("Invalid filepath");
        self.filepath = Some(filepath.into());
        let mut data = String::new();
        for (i, line) in (&self.lines).iter().enumerate() {
            data.push_str(&line.buffer.clone().join(""));
            if i + 1 != self.lines.len() { data.push('\n') }
        }
        fs::write(valid_filepath, &data).expect(&format!("Unable to write file to {}", filepath));
    }

    /// Ask for the filepath to load
    pub fn load(&mut self) {
        let recent_files = self.get_recent_files();
        let mut menu_items = vec![MenuItem::new("Open ...".into(), MenuAction::Void)];
        for (name, path) in recent_files {
            menu_items.push(MenuItem::new(&name, MenuAction::Open(path)));
        }
        self.menu.open_with(menu_items);
    }

    /// Load a specific path
    pub fn load_file(&mut self, filepath: &str) {
        // TODO: check that the path is valid
        let valid_filepath = fs::canonicalize(filepath).expect("Invalid filepath");
        self.lines = vec![Line::new(Rc::clone(&self.font))];
        self.underline_buffer = vec![];
        self.bold_buffer = vec![];
        self.selection.reset();
        self.filepath = Some(filepath.into());
        let file_content = fs::read_to_string(&valid_filepath).expect(&format!("Unable to load file to {}", filepath));
        for (i, line) in file_content.split('\n').enumerate() {
            if i < self.lines.len() {
                self.lines.push(Line::new(Rc::clone(&self.font)));
            }
            self.lines[i].add_text(line);
        }
        self.cursor.move_to(0, 0);
        self.update_text_layout();
    }

    pub fn update_text_layout(&mut self) {
        let mut difference = 0;
        for (i, line) in (&mut self.lines).iter_mut().enumerate() {
            let diff = line.update_text_layout();
            if i as u32 == self.cursor.y { difference = diff; }
        }
        self.font.borrow_mut().style_changed = false;
        self.cursor.move_to((self.cursor.x as i32 - difference) as u32, self.cursor.y);
    }

    pub fn update(&mut self, dt: f32) {
        let animations = [
            &mut self.cursor.animation.x, &mut self.cursor.animation.y,
            &mut self.camera.animation.x,  &mut self.camera.animation.y,
            &mut self.menu.size_animation.x,  &mut self.menu.size_animation.y, &mut self.menu.focus_y_animation
        ];
        for animation in animations {
            if let Some(anim) = animation {
                if !anim.has_started {
                    anim.start();
                }
                anim.update(dt);
                if anim.is_ended {
                    *animation = Option::None;
                }
            }
        }
    }

    pub fn render(&mut self, graphics: &mut Graphics2D) {
        let char_width = self.font.borrow().char_width;
        let char_height = self.font.borrow().char_height;

        let mut previous_line_height = 0.;
        // Draw text
        for (i, line) in self.lines.iter().enumerate() {
            line.render(
                - self.camera.computed_x(),
                - self.camera.computed_y() + previous_line_height * (i as f32),
                graphics,
            );
            previous_line_height = if line.formatted_text_block.height() > 0. {
                line.formatted_text_block.height()
            } else {
                line.font.borrow().char_height
            };
        }

        // Specific line camera which derive of the global one to handle text alignement
        let line_offset = self.get_current_line().alignement_offset;
        let line_camera = Camera::from_with_offset(&self.camera, Vector2::new(-line_offset, 0.));

        // draw underline
        for range in &mut self.underline_buffer {
            assert!(range.is_valid());
            let line = &self.lines[range.start.unwrap().y as usize];
            let line_offset = line.alignement_offset;
            let line_camera = Camera::from_with_offset(&self.camera, Vector2::new(-line_offset, 0.));
            let lines_index = range.get_lines_index(&self.lines);
            let initial_y = range.get_start_y();
            for (i, (start, end)) in lines_index.iter().enumerate() {
                let y = (initial_y as usize + i) as f32 * char_height;
                graphics.draw_line(
                    Vector2::new(*start as f32 * char_width - line_camera.computed_x(), y + 0.9 * char_height - line_camera.computed_y()),
                    Vector2::new(*end as f32 * char_width - line_camera.computed_x(), y + 0.9 * char_height - line_camera.computed_y()),
                    1.,
                    Color::BLACK
                );
            }
        }
        // self.camera._render(graphics);
        self.cursor.render(&line_camera, graphics);
        self.selection.render(Rc::clone(&self.font), &self.lines, &self.camera, graphics);
        self.menu.render(&self.cursor, &self.camera, graphics);
        graphics.draw_rectangle( // draw the title bar
            Rectangle::new(
                Vector2::new(0., 0.),
                Vector2::new(self.camera.width, EDITOR_OFFSET_TOP),
            ),
            Color::WHITE
        );
        // draw the title bar line
        if self.camera.computed_y() > EDITOR_PADDING + EDITOR_OFFSET_TOP {
            graphics.draw_line(
                Vector2::new(0., EDITOR_OFFSET_TOP),
                Vector2::new(self.camera.width, EDITOR_OFFSET_TOP),
                0.5,
                Color::GRAY
            );
        }
    }
}
