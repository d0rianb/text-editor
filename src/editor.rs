use std::{cmp, env, fs};
use std::cell::RefCell;
use std::rc::Rc;
use std::path::{Path, PathBuf};
use std::time::Instant;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::font::TextAlignment;
use speedy2d::Graphics2D;
use speedy2d::shape::Rectangle;
use speedy2d::window::{ModifiersState, UserEventSender, VirtualKeyCode};

use clipboard::ClipboardProvider;
use clipboard::ClipboardContext;
use lazy_static::lazy_static;
use regex::Regex;
use ifmt::iformat;

use serde_yaml;

use crate::cursor::{Cursor, CURSOR_OFFSET_X};
use crate::camera::Camera;
use crate::contextual_menu::{ContextualMenu, MenuItem};
use crate::{Animation, EditorEvent, FocusElement, MenuId};
use crate::menu_actions::MenuAction;
use crate::font::Font;
use crate::line::Line;
use crate::range::Range;
use crate::selection::Selection;
use crate::editable::Editable;
use crate::stats::Stats;

pub const EDITOR_PADDING: f32 = 10.;
pub const EDITOR_OFFSET_TOP: f32 = 55.;


pub struct Editor {
    pub lines: Vec<Line>,
    pub cursor: Cursor,
    pub camera: Camera,
    pub offset: Vector2<f32>,
    pub padding: f32,
    pub font: Rc<RefCell<Font>>,
    pub system_font: Rc<RefCell<Font>>,
    pub modifiers : ModifiersState,
    pub filepath: Option<String>,
    pub event_sender: Option<UserEventSender<EditorEvent>>,
    pub selection: Selection,
    pub underline_buffer: Vec<Range>,
    pub bold_buffer: Vec<Range>,
    pub menu: ContextualMenu,
    pub cached_prefs: Option<serde_yaml::Value>,
    pub stats: Stats,
    pub should_edit_file: bool, // so the input does not trigger file specific events
}

impl Editor {
    pub fn new(width: f32, height: f32, offset: Vector2<f32>, padding: f32) -> Self {
        let font = Rc::new(RefCell::new(Font::new(
            &Self::get_file_path("./resources/font/CourierRegular.ttf"),
            width - offset.x - padding * 2.,
            height - offset.y - padding * 2.,
        )));
        let system_font = Rc::new(RefCell::new(Font::new(&Self::get_file_path("./resources/font/Roboto-Regular.ttf"), width, height)));
        Self {
            cursor: Cursor::new(0, 0, Rc::clone(&font)),
            camera: Camera::new(width, height, offset, padding),
            lines: vec![Line::new(Rc::clone(&font))],
            selection: Selection::new(Rc::clone(&font)),
            system_font: system_font.clone(),
            modifiers: ModifiersState::default(),
            filepath: Option::None,
            event_sender: Option::None,
            underline_buffer: vec![],
            bold_buffer: vec![],
            menu: ContextualMenu::new(system_font),
            cached_prefs: Option::None,
            offset,
            padding,
            font,
            stats: Stats::default(),
            should_edit_file: true
        }
    }
}

impl Editable for Editor {
    fn add_char(&mut self, c: String) {
        if self.modifiers.logo() {
            let chars: Vec<char> = c.chars().collect();
            return self.shortcut(chars[0]);
        }
        // matching template
        let mut after = "";
        for template in [("(", ")"), ("[", "]"), ("{", "}"), ("\"", "\"")] {
            if c == template.0 { after = template.1; break }
        }
        if after == "" { self.delete_selection(); }
        let pos = if self.selection.is_valid() { self.selection.start().unwrap() } else { Vector2::new(self.cursor.x, self.cursor.y) };
        self.get_current_buffer().insert(pos.x as usize, c);
        if after != "" {
            let after_pos = if self.selection.is_valid() { self.selection.end().unwrap() } else { Vector2::new(self.cursor.x, self.cursor.y) };
            self.lines[after_pos.y as usize].buffer.insert(after_pos.x as usize + 1, after.into());
        }
        self.set_dirty(true);
        self.move_cursor_relative(1, 0);
        self.selection.reset();
    }

    fn delete_char(&mut self) {
        if self.modifiers.alt() || self.modifiers.logo()  {
            self.begin_selection();
            self.move_cursor_relative(-1, 0);
            self.end_selection();
        }
        self.set_dirty(true);
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
            if buffer.len() as i32 > pos && buffer.get(pos as usize - 1) == buffer.get(pos as usize)
                && ["(", "[", "{", "\""].contains(&buffer.get(pos as usize - 1).unwrap().as_str()) {
                buffer.remove(pos as usize);
            }
            buffer.remove(pos as usize - 1);
            self.move_cursor_relative(-1, 0);
        }
        self.selection.reset();
        self.update_text_layout();
    }

    fn handle_key(&mut self, keycode: VirtualKeyCode) {
        let ctrl_alt = self.modifiers.logo() && self.modifiers.alt();

        match keycode {
            VirtualKeyCode::Right => if ctrl_alt { self.set_line_alignment(TextAlignment::Right) } else { self.move_cursor_relative(1, 0) },
            VirtualKeyCode::Left => if ctrl_alt { self.set_line_alignment(TextAlignment::Left) } else { self.move_cursor_relative(-1, 0) },
            VirtualKeyCode::Up => if ctrl_alt { self.set_line_alignment(TextAlignment::Center) } else { self.move_cursor_relative(0, -1) },
            VirtualKeyCode::Down => self.move_cursor_relative(0, 1),
            VirtualKeyCode::Backspace => self.delete_char(),
            VirtualKeyCode::Delete => { self.move_cursor_relative(1, 0); self.delete_char(); },
            VirtualKeyCode::Return => if self.modifiers.alt() { self.toggle_ai_contextual_menu() } else { self.new_line() },
            VirtualKeyCode::Escape => self.menu.close(),
            VirtualKeyCode::Tab => if self.modifiers.alt() { self.menu.open() },
            _ => { return; },
        }
        self.update_text_layout();
    }

    fn move_cursor(&mut self, position: Vector2<u32>) {
        assert!(!self.lines.is_empty());
        let pos = self.get_valid_cursor_position(position);
        if pos.x != self.cursor.x || pos.y != self.cursor.y {
            self.cursor.move_to(pos.x, pos.y);
        }
        self.update_camera();
    }

    fn move_cursor_relative(&mut self, rel_x: i32, rel_y: i32) {
        let max_y = self.lines.len() as i32 - 1;
        let mut new_x = (self.cursor.x as i32 + rel_x) as i32;
        let mut new_y = (self.cursor.y as i32 + rel_y).clamp(0, max_y);

        if self.modifiers.shift() && self.selection.start().is_none() {
            self.selection.set_start(Vector2::new(self.cursor.x, self.cursor.y));
        }

        if self.modifiers.alt() {  // Move to the previous/next word
            let (start, end) = self.lines[self.cursor.y as usize].get_next_jump(self.cursor.x);
            if rel_x < 0 && start != self.cursor.x  {
                new_x = start as i32;
            } else if rel_x > 0 && end != self.cursor.x  {
                new_x = end as i32;
            }
        } else if self.modifiers.logo() { // Move to the start/end of the line/file
            if self.modifiers.ctrl() {
                self.switch_lines(rel_y)
            } else {
                if rel_x < 0  { new_x = 0; }
                else if rel_x > 0 { new_x = self.lines[self.cursor.y as usize].buffer.len() as i32; }
                if rel_y < 0 { new_y = 0; }
                else if rel_y > 0 { new_y = self.lines.len() as i32 - 1; }
            }
        }

        if self.selection.is_valid() && !self.modifiers.shift() && !(self.modifiers.logo() && self.modifiers.ctrl()) { // go to the start/end of the selection
            if rel_x > 0 || rel_y > 0 {
                self.move_cursor(self.selection.end().unwrap());
                self.selection.reset();
                return;
            } else if rel_x < 0 || rel_y < 0 {
                self.move_cursor(self.selection.start().unwrap());
                self.selection.reset();
                return;
            }
        }

        if new_x < 0 {  // Go to line before
            if self.cursor.y == 0 { return; }
            let previous_line_buffer_size = self.lines[self.cursor.y as usize - 1].buffer.len() as u32;
            self.cursor.move_to(previous_line_buffer_size, self.cursor.y - 1);
        } else if new_x as usize > self.get_current_buffer().len() { // Go to line after
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
            self.selection.set_end(Vector2::new(self.cursor.x, self.cursor.y));
        } else if (rel_x.abs() > 0 || rel_y.abs() > 0) && self.selection.is_valid() && !(self.modifiers.logo() && self.modifiers.ctrl()) {
            self.selection.reset();
        }
        self.update_camera();
    }

    fn shortcut(&mut self, c: char) {
        match c {
            's' => self.save(),
            'S' => self.toggle_save_popup(),
            'o' => self.load(),
            'u' => self.underline(),
            'c' => self.copy(),
            'v' => self.paste(),
            'x' => { self.copy(); self.delete_selection() },
            'a' => self.select_all(),
            'l' => self.select_current_line(),
            'L' => { self.select_current_line(); self.delete_selection() },
            'w' | 'q' => self.quit(),
            'd' => self.select_current_word(),
            'D' => self.duplicate_line(),
            '+' | '=' => self.increase_font_size(),
            '-' => self.decrease_font_size(),
            'n' => self.new_file_popup(),
            'N' => self.new_file("new-file.txt"),
            'i' => self.toggle_stats_popup(),
            'r' => self.find_next(),
            'p' => self.print_dir(),
            'P' => self.toggle_ai_contextual_menu(),
            _ => {}
        }
    }

    fn begin_selection(&mut self) {
        self.selection.set_start((self.cursor.x, self.cursor.y).into());
    }

    fn end_selection(&mut self) {
        self.selection.set_end((self.cursor.x, self.cursor.y).into());
    }

    fn update_selection(&mut self, position: Vector2<f32>) {
        let mouse_position = self.get_mouse_position_index(position);
        if let Some(end) = &self.selection.end() {
            if mouse_position == *end { return; }
        }
        self.selection.set_end(mouse_position);
        self.move_cursor(mouse_position);
    }

    fn delete_selection(&mut self) {
        if self.selection.is_valid() {
            let initial_i = cmp::min(self.selection.start().unwrap().y, self.selection.end().unwrap().y) as usize;
            let lines_indices = self.selection.get_lines_index(&self.lines);
            for (i, indices) in lines_indices.iter().enumerate() {
                let start = cmp::min(indices.0, indices.1) as usize;
                let end = cmp::max(indices.0, indices.1) as usize;
                let _ = &self.lines[initial_i + i].buffer.drain(start .. end);
            }
            for i in 0 .. lines_indices.len() {
                let index = initial_i + lines_indices.len() - i - 1;
                if self.lines[index].buffer.is_empty() && self.lines.len() > 1 {
                    self.lines.remove(index);
                }
            }
            let selection_start = self.selection.start().unwrap();
            self.move_cursor(Vector2::new(selection_start.x, selection_start.y));
            self.selection.reset();
        }
    }

    fn get_mouse_position_index(&mut self, position: Vector2<f32>) -> Vector2<u32> {
        let pos = Vector2::new(
            ((position.x as f32 + self.camera.computed_x()) / self.font.borrow().char_width + 0.5) as u32,
            ((position.y as f32 + self.camera.computed_y()) / self.font.borrow().char_height) as u32,
        );
        self.get_valid_cursor_position(pos)
    }

    fn get_valid_cursor_position(&mut self, position: Vector2<u32>) -> Vector2<u32> {
        let max_y = self.lines.len() as u32 - 1;
        let y = cmp::min(position.y, max_y);
        let line = &self.lines[y as usize];
        let x =  (position.x as i32 - (line.alignment_offset / self.font.borrow().char_width + 0.5) as i32).clamp(0, line.buffer.len() as i32) as u32;
        Vector2::new(x, y)
    }

    fn select_current_word(&mut self) {
        let (start, end) = self.lines[self.cursor.y as usize].get_word_at(self.cursor.x);
        self.selection.set(
            Vector2::new(start, self.cursor.y),
            Vector2::new(end, self.cursor.y),
        )
    }

    fn select_all(&mut self) {
        let last_line_length = self.lines.last().unwrap().buffer.len() as u32;
        self.selection.set(Vector2::ZERO, Vector2::new(last_line_length, self.lines.len() as u32 - 1));
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

    fn copy(&mut self) {
        if !self.selection.is_valid() { return; }
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        let selection_text = self.get_selected_text();
        ctx.set_contents(selection_text).unwrap();
    }

    fn paste(&mut self) {
        if self.selection.is_valid() { self.delete_selection(); }
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        let clipboard_content = ctx.get_contents().unwrap();
        if clipboard_content.is_empty() { return; }
        let mut lines = clipboard_content.split('\n').filter(|c| *c != "");
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
        self.set_dirty(true);
    }
}

impl Editor {
    fn quit(&mut self) {
        if let Some(_filepath) = &mut self.filepath {  } else { self.filepath = Some("newfile.txt".into()); }
        self.save();
        std::process::exit(0)
    }

    pub fn set_event_sender(&mut self, es: Option<UserEventSender<EditorEvent>>) {
        self.event_sender = es.clone();
        self.cursor.event_sender = es.clone();
        self.selection.event_sender = es.clone();
        self.camera.event_sender = es.clone();
        self.menu.event_sender = es.clone();
    }

    fn send_event(&self, event: EditorEvent) {
        match event {
            EditorEvent::SetDirty(_, _) | EditorEvent::LoadFile(_) => if !self.should_edit_file { return; }
            _ => {}
        }
        self.event_sender.as_ref().unwrap().send_event(event).unwrap();
    }

    pub fn set_dirty(&mut self, dirty: bool) {
        let path = self.filepath.clone().unwrap_or(String::from(""));
        self.send_event(EditorEvent::SetDirty(path, dirty)); // Set the editor dirty
    }

    pub fn set_offset(&mut self, offset: Vector2<f32>) {
        self.offset = offset;
        let width = self.system_font.borrow().editor_size.x; // Hack to get the original width back
        let height = self.system_font.borrow().editor_size.y; // Hack to get the original height back
        self.camera = Camera::new(width, height, offset, self.padding);
        self.camera.event_sender = self.event_sender.clone();
        self.font.borrow_mut().editor_size.x = width - offset.x - self.padding * 2.;
        self.font.borrow_mut().editor_size.y = height - offset.y - self.padding * 2.;
    }

    pub fn on_resize(&mut self, size: Vector2<u32>) {
        self.system_font.borrow_mut().on_resize(size);
        self.camera.on_resize(size);
        self.font.borrow_mut().on_resize(size);
    }

    fn get_valid_cursor_position(&mut self, position: Vector2<u32>) -> Vector2<u32> {
        let max_y = self.lines.len() as u32 - 1;
        let y = cmp::min(position.y, max_y);
        let line = &self.lines[y as usize];
        let x =  (position.x as i32 - (line.alignment_offset / self.font.borrow().char_width + 0.5) as i32).clamp(0, line.buffer.len() as i32) as u32;
        Vector2::new(x, y)
    }

    pub fn update_camera(&mut self) {
        // Horizontal Scroll
        if self.camera.get_cursor_x_with_offset(&self.cursor) < self.camera.computed_x() + self.camera.safe_zone_size {
            self.camera.move_x(self.camera.get_cursor_x_with_offset(&self.cursor) - self.camera.computed_x() - self.camera.safe_zone_size);
        } else if self.padding + self.cursor.real_x() - self.camera.computed_x() > self.camera.width - self.camera.safe_zone_size {
            self.camera.move_x(self.padding + self.cursor.real_x() - self.camera.computed_x() - self.camera.width + self.camera.safe_zone_size);
        }
        // Vertical Scroll
        if self.camera.get_cursor_y_with_offset(&self.cursor) < self.camera.computed_y() + self.camera.safe_zone_size {
            self.camera.move_y(self.camera.get_cursor_y_with_offset(&self.cursor) - self.camera.computed_y() - self.camera.safe_zone_size)
        } else if self.padding + self.cursor.real_y() - self.camera.computed_y() > self.camera.height - self.camera.safe_zone_size {
            self.camera.move_y(self.padding + self.cursor.real_y() - self.camera.computed_y() - self.camera.height + self.camera.safe_zone_size)
        }
    }

    fn get_current_line(&mut self) -> &mut Line {
        &mut self.lines[self.cursor.y as usize]
    }

    fn get_current_buffer(&mut self) -> &mut Vec<String> {
        &mut self.get_current_line().buffer
    }

    pub fn get_selected_text(&mut self) -> String {
        if !self.selection.is_valid() { return String::new() }
        let mut buffer = vec![];
        let lines_index = self.selection.get_lines_index(&self.lines);
        let initial_y = self.selection.start().unwrap().y;
        for (i, (start, end)) in lines_index.iter().enumerate() {
            let y = initial_y as usize + i;
            let mut text = String::new();
            for j in *start .. *end {
                let buffer_text = self.lines[y].buffer.get(j as usize);
                if let Some(bt) = buffer_text { text.push_str(bt); }
            }
            text.push('\n');
            buffer.push(text);
        }
        buffer.join("")
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
            self.move_cursor(Vector2::new(0, self.cursor.y + 1));
            return;
        }
        let line_before_buffer= self.lines.get(index - 1).unwrap().buffer.clone();
        let line_before_alignement = self.lines.get(index - 1).unwrap().alignment.clone();
        let last_line = self.lines.get_mut(index).unwrap();
        last_line.set_alignment(line_before_alignement); // Preserve the alignement
        let text = line_before_buffer.join("");
        let nb_whitespace = text.len() - text.trim_start().len();
        if text.trim_start().starts_with('-') &&  text.trim().len() > 1 {
            let new_text = " ".repeat(nb_whitespace) + "- "; // TODO: Aadapt the number of spaces after the dash
            last_line.add_text(&new_text);
            self.move_cursor(Vector2::new(nb_whitespace as u32 + 2, self.cursor.y + 1));
            self.menu.open_with(vec![MenuItem::new("Annuler", MenuAction::CancelChip) ]);
        } else {
            self.move_cursor(Vector2::new(0, self.cursor.y + 1));
        }
    }

    fn duplicate_line(&mut self) {
        let cursor_pos = Vector2::new(self.cursor.x, self.cursor.y);
        let index_start = self.selection.start().unwrap_or(cursor_pos).y as usize;
        let index_end = self.selection.end().unwrap_or(cursor_pos).y as usize;
        let line_slice = self.lines[index_start..=index_end].to_vec();
        for (i, line) in line_slice.iter().enumerate() {
            self.lines.insert(index_start + i, (*line).clone());
        }
        self.move_cursor(Vector2::new(self.cursor.x, self.cursor.y + line_slice.len() as u32))
    }

    fn switch_lines(&mut self, dir: i32) {
        let cursor_pos = Vector2::new(self.cursor.x, self.cursor.y);
        let index_start = self.selection.start().unwrap_or(cursor_pos).y as usize;
        let index_end = self.selection.end().unwrap_or(cursor_pos).y as usize;
        if dir < 0 {
            for i in index_start..=index_end { self.lines.swap(i, (i as i32 - 1).abs() as usize); }
        } else if dir > 0 {
            for i in 0..=(index_end - index_start) { self.lines.swap(index_end - i, index_end - i as usize + 1); }
        }
        if self.selection.is_valid() {
            self.selection.set_start(Vector2::new(self.selection.start().unwrap().x, (self.selection.start().unwrap().y as i32 + dir) as u32));
            self.selection.set_end(Vector2::new(self.selection.end().unwrap().x, (self.selection.end().unwrap().y as i32 + dir) as u32));
        }
        self.move_cursor(Vector2::new(self.cursor.x, (self.cursor.y as i32 + dir) as u32))
    }

    pub fn add_text(&mut self, text: &str) {
        for c in text.chars() {
            self.add_char(c.to_string());
        }
    }

    pub fn cancel_chip(&mut self) {
        self.get_current_line().empty();
        self.cursor.move_to(0, self.cursor.y);
        self.new_line(); // Move the cursor to the new created line
        self.update_text_layout();
    }

    pub fn toggle_contextual_menu(&mut self) {
        let mut items = vec![];
        if self.selection.is_valid() {
            for i in [
                MenuItem::new("Copy", MenuAction::Copy),
                MenuItem::new("Cut", MenuAction::Cut),
                MenuItem::new("Paste", MenuAction::Paste),
                MenuItem::separator(),
                MenuItem::new("Bold", MenuAction::Bold),
                MenuItem::new("Underline", MenuAction::Underline),
            ] { items.push(i) }
        } else {
            return self.toggle_save_popup();
        }
        self.menu.open_with(items);
    }

    pub fn toggle_ai_contextual_menu(&mut self) {
        if !self.selection.is_valid() { self.select_current_word(); }
        self.menu.open_with(vec![
            MenuItem::new("AI Correct", MenuAction::AICorrect),
            MenuItem::new("AI Action >", MenuAction::AIQuestionWithInput),
        ]);
    }

    pub fn get_menu(&mut self, id: MenuId) -> &mut ContextualMenu {
        // MenuId example: (0, -1, -1, -1) | (1, 3, -1, -1)
        let mut menu = &mut self.menu;
        for level in id.iter() {
            if *level <= -1 || *level as usize >= menu.items.len() { break; }
            menu  = menu.items[*level as usize].sub_menu.as_mut().unwrap();
        }
        menu
    }

    fn get_focus_menu_id(&mut self) -> Option<MenuId> {
        if !self.menu.is_focus() { return Option::None }
        let mut id = self.menu.id;
        let mut last_menu_focused = false;
        'menus: while !last_menu_focused {
            let menu = self.get_menu(id);
            let items_submenu =  menu.items.iter().map(|i| i.sub_menu.as_ref()).clone();
            #[warn(unused_labels)]
            'items: for (i, sub_menu) in items_submenu.enumerate() {
                if let Some(sub_menu) = sub_menu {
                    if sub_menu.is_focus() {
                        let item = &menu.items[i];
                        id = item.sub_menu.as_ref().unwrap().id;
                        continue 'menus;
                    }
                }
            }
            last_menu_focused = true
        }
        Some(id)
    }

    pub fn get_focus_menu(&mut self) -> Option<&mut ContextualMenu> {
        let menu_id = self.get_focus_menu_id();
        return if let Some(id) = &menu_id {
            Some(self.get_menu(*id))
        } else {
            None
        }
    }

    fn _contextual_submenu_test(&mut self) {
        let sub_menu = ContextualMenu::new_with_items(self.system_font.clone(), self.event_sender.clone().unwrap(), vec![
            MenuItem::new("SubMenu 1", MenuAction::Void),
            MenuItem::new("SubMenu Input", MenuAction::PrintWithInput),
            MenuItem::new("SubMenu 3", MenuAction::Void),
            MenuItem::new("SubMenu 4", MenuAction::Void),
        ]);
        self.menu.open_with(vec![
            MenuItem::new("Menu 1", MenuAction::Underline),
            MenuItem::new("New ...", MenuAction::PrintWithInput),
            MenuItem {
                title: "Menu 2 >".to_string(),
                action: MenuAction::OpenSubMenu,
                sub_menu: Some(sub_menu),
                input: Option::None,
                loader: None
            }
        ]);
    }

    #[cfg(debug_assertions)]
    fn get_working_dir() -> PathBuf { env::current_dir().unwrap() }

    #[cfg(not(debug_assertions))]
    fn get_working_dir() -> PathBuf {
        let path_buf = env::current_exe().unwrap();
        path_buf.parent().unwrap().to_path_buf()
    }

    pub fn get_file_path(filename: &str) -> String {
        let mut wd = Self::get_working_dir();
        wd.push(filename);
        let valid_file_path = wd.canonicalize().expect(&format!("Invalid path : {:?}", wd));
        valid_file_path.into_os_string().to_str().unwrap().to_string()
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

    pub fn underline(&mut self) {
        Self::add_range_to_buffer(self.selection.get_range(), &mut self.underline_buffer);
        self.set_dirty(true);
    }

    pub fn bold(&mut self) {
        Self::add_range_to_buffer(self.selection.get_range(), &mut self.bold_buffer);
        self.set_dirty(true);
    }

    pub fn set_line_alignment(&mut self, alignment: TextAlignment) {
        if self.selection.is_valid() {
            let start = self.selection.start().unwrap().y as usize;
            let end = self.selection.end().unwrap().y as usize;
            for (i, line) in self.lines.iter_mut().enumerate() {
                if start <= i && i <= end {
                    line.set_alignment(alignment.clone());
                }
            }
        } else {
            self.get_current_line().set_alignment(alignment);
        }
        self.set_dirty(true);
    }

    fn increase_font_size(&mut self) {
        self.font.borrow_mut().change_font_size(2);
        self.update_text_layout();
        self.update_camera();
        self.set_dirty(true);
        self.send_event(EditorEvent::Redraw);
    }

    fn decrease_font_size(&mut self) {
        self.font.borrow_mut().change_font_size(-2);
        self.update_text_layout();
        self.update_camera();
        self.set_dirty(true);
        self.send_event(EditorEvent::Redraw);
    }

    fn find_next(&mut self) {
        self.menu.open_with(vec![MenuItem::new("Find:", MenuAction::FindAndJumpWithInput)])
    }

    pub fn find(&mut self, text: &str) {
        let cursor_y = self.cursor.y as usize;
        for i in 0 .. self.lines.len() {
            let line_index = (i + cursor_y) % self.lines.len(); // begin the search at cursor.y then loop
            let start = if line_index as u32 == self.cursor.y { self.cursor.x as usize } else { 0 };
            let match_index = &self.lines[line_index].get_text()[start..].find(text); //.map(|i| i);
            if let Some(index) = match_index {
                self.selection.reset();
                self.move_cursor(Vector2::new((start + index) as u32, line_index as u32));
                break;
            }
        }
    }

    fn get_stats(&self) -> Vec<String> {
        // TODO: get stats of selection if there is one instead of the whole file
        let words_count = self.lines.iter().fold(0, |acc, line| acc + line.get_word_count());
        let char_count = self.lines.iter().fold(0, |acc, line| acc + line.buffer.len());
        let update_duration = self.stats.update_duration.as_micros() as f64 / 1000.;
        let draw_duration = self.stats.draw_duration.as_micros() as f64 / 1000.;
        vec![
            iformat!("Nombre de mots: {words_count}"),
            iformat!("Nombre de caractères: {char_count}"),
            iformat!("Nombre de lignes: {self.lines.len()}"),
            iformat!("Position du curseur: ({self.cursor.x}, {self.cursor.y})"),
            iformat!("---"),
            iformat!("Update time: {update_duration:.1}ms"),
            iformat!("Draw time: {draw_duration:.1}ms"),
        ]
    }

    fn toggle_stats_popup(&mut self) {
        if self.menu.is_visible { return self.menu.close(); }
        self.menu.open_with(self.get_stats().iter().map(|s| {
            if s.starts_with("---") { return MenuItem::separator() }
            MenuItem::new(s, MenuAction::Information)
        }).collect());
        self.send_event(EditorEvent::Focus(FocusElement::Editor));
    }

    fn get_prefs_key(&mut self, key: &str) -> serde_yaml::Value {
        if let Some(prefs) = &self.cached_prefs {
            prefs.get(key).unwrap().to_owned()
        } else {
            let prefs_path = Self::get_file_path("./resources/prefs.yaml");
            let prefs_str = fs::read_to_string(prefs_path).expect("Can't find the preference file");
            let prefs: serde_yaml::Value = serde_yaml::from_str(&prefs_str).expect("Invalid preferences");
            self.cached_prefs = Some(prefs.clone());
            prefs.get(key).unwrap().to_owned()
        }
    }

    fn set_prefs_key(&mut self, key: &str, value: serde_yaml::Value) {
        let mut prefs = if let Some(prefs) = &self.cached_prefs { prefs.to_owned() } else {
            let prefs_path = Self::get_file_path("./resources/prefs.yaml");
            let prefs_str = fs::read_to_string(prefs_path).expect("Can't find the preference file");
            let prefs: serde_yaml::Value = serde_yaml::from_str(&prefs_str).expect("Invalid preferences");
            prefs
        };
        *prefs.get_mut(key).unwrap() = value.into();
        let mut buffer = Vec::new();
        serde_yaml::to_writer(&mut buffer, &prefs).unwrap();
        fs::write(Self::get_file_path("./resources/prefs.yaml"), buffer).expect("Unable to write to the preference file");
        self.cached_prefs = Option::None;
    }

    fn get_recent_files(&mut self) -> Vec<(String, String)> {
        lazy_static! { static ref NAME_REGEX: Regex = Regex::new(r#"([\w\s_-]+).(\w+)$"#).unwrap(); }
        let files_yaml = self.get_prefs_key("recent_files");
        let files: Vec<&str> = files_yaml.as_sequence().unwrap().iter().map(|f| f.as_str().unwrap()).collect();
        let files_with_names: Vec<(String, String)> = files.iter().map(|f| {
            let file_name: String = NAME_REGEX.captures(*f).unwrap().get(0).unwrap().as_str().to_string();
            (file_name, String::from(*f))
        }).collect();
        files_with_names
    }

    fn get_recent_paths(&mut self) -> Vec<(String, String)> {
        lazy_static! { static ref NAME_REGEX: Regex = Regex::new(r"(\w+)/?$").unwrap(); }
        let folder_yaml = self.get_prefs_key("recent_folders");
        let folder: Vec<&str> = folder_yaml.as_sequence().unwrap().iter().map(|f| f.as_str().unwrap()).collect();
        let folder_with_names: Vec<(String, String)> = folder.iter().map(|f| {
            let file_name: String = NAME_REGEX.captures(*f).unwrap().get(0).unwrap().as_str().to_string() + "/";
            (file_name, String::from(*f) + "/")
        }).collect();
        folder_with_names
    }

    fn add_to_recent_files(&mut self, filepath: &str) {
        const MAX_ELEMENT: usize = 3;
        let recent_files = self.get_recent_files();
        let mut existing_filepaths: Vec<&str> = recent_files.iter().map(|(_name, path)| path.as_str()).collect();
        if let Some(index) = &existing_filepaths.iter().position(|f| f == &filepath) { existing_filepaths.remove(*index); }
        existing_filepaths.insert(0, filepath);
        existing_filepaths.truncate(MAX_ELEMENT);
        let yaml_array = serde_yaml::Value::Sequence(existing_filepaths.iter().map(|f| serde_yaml::Value::String((*f).to_string())).collect());
        self.set_prefs_key("recent_files", yaml_array);
    }

    fn add_to_recent_paths(&mut self, filepath: &str) {
        lazy_static! {
            static ref NAME_REGEX: Regex = Regex::new(r"(\w+)/?$").unwrap();
        }
        const MAX_ELEMENT: usize = 5;
        let path = Path::new(filepath).parent().unwrap().as_os_str().to_str().unwrap().to_string() + "/";
        let recent_paths = self.get_recent_paths();
        let mut existing_paths: Vec<&String> = recent_paths.iter().map(|(_name, path)| path).collect();
        if let Some(index) = &existing_paths.iter().position(|f| *f == &path) { existing_paths.remove(*index); }
        existing_paths.insert(0, &path);
        existing_paths.truncate(MAX_ELEMENT);
        let yaml_array = serde_yaml::Value::Sequence(existing_paths.iter().map(|f| {
            let mut name = (*f).to_owned();
            if name.ends_with('/') { name.pop(); }
            serde_yaml::Value::String(name)
        }).collect());
        self.set_prefs_key("recent_folders", yaml_array);
    }

    fn new_file_popup(&mut self) {
        let mut path_items = vec![];
        for (name, path) in self.get_recent_paths() {
            path_items.push(MenuItem::new(&name, MenuAction::NewFileWithInput(path)));
        }
        let new_file_submenu = ContextualMenu::new_with_items(self.system_font.clone(), self.event_sender.clone().unwrap(), path_items);
        let items = vec![
            MenuItem::new("New Empty File", MenuAction::NewFile("new-file.txt".into())),
            MenuItem::new_with_submenu("New File ...", new_file_submenu),
        ];
        self.menu.open_with(items);
    }

    pub fn new_file(&mut self, path: &str) {
        self.select_all();
        self.delete_selection();
        self.save_to_file(path);
        self.load_file(path);
    }

    fn print_dir(&mut self) {
        let current_exe = env::current_exe();
        let text =  current_exe
            .as_ref()
            .unwrap()
            .to_str()
            .unwrap();
        self.lines
            .get_mut(0)
            .unwrap()
            .add_text(text);
    }

    /// Ask for the filepath if there is no one specified else save to the current one
    pub fn save(&mut self) {
        if let Some(f) = self.filepath.clone() {
            if &f == "new-file.txt" { return self.toggle_save_popup(); }
            self.save_to_file(&f);
        } else {
            self.toggle_save_popup()
        }
    }

    fn toggle_save_popup(&mut self) {
        let mut path_items = vec![];
        for (name, path) in self.get_recent_paths() {
            path_items.push(MenuItem::new(&name, MenuAction::SaveWithInput(path)));
        }
        let path_submenu = ContextualMenu::new_with_items(self.system_font.clone(), self.event_sender.clone().unwrap(), path_items);
        let mut file_items = vec![MenuItem::new_with_submenu("Save to >", path_submenu), MenuItem::separator()];
        for (name, path) in self.get_recent_files() {
            file_items.push(MenuItem::new(&name, MenuAction::Save(path)));
        }
        self.menu.open_with(file_items);
    }

    fn get_valid_path_or_create_it(&self, filepath: &str) -> PathBuf {
        let path = Path::new(filepath);
        if let Some(prefix) = path.parent() { fs::create_dir_all(prefix).unwrap(); }
        if !path.is_file() { fs::File::create(path).unwrap(); }
        fs::canonicalize(path).expect("Invalid filepath")
    }

    /// Save to a specific file
    pub fn save_to_file(&mut self, filepath: &str) {
        if filepath.ends_with(".txt") { self.save_to_txt_file(filepath) }
        else if filepath.ends_with(".drn") { self.save_to_drn_file(filepath) }
        self.set_dirty(false);
    }

    pub fn save_to_txt_file(&mut self, filepath: &str) {
        let valid_filepath = self.get_valid_path_or_create_it(filepath);
        self.filepath = Some(filepath.into());
        let mut data = String::new();
        for (i, line) in (&self.lines).iter().enumerate() {
            data.push_str(&line.buffer.clone().join(""));
            if i + 1 != self.lines.len() { data.push('\n') }
        }
        fs::write(valid_filepath, &data).expect(&format!("Unable to write file to {}", filepath));
        self.send_event(EditorEvent::LoadFile(filepath.into()))
    }

    pub fn save_to_drn_file(&mut self, filepath: &str) {
        let valid_filepath = self.get_valid_path_or_create_it(filepath);
        self.filepath = Some(filepath.into());
        let mut encode = String::new();
        // Encode underline
        encode.push_str("#u: ");
        let underline_ranges = self.underline_buffer
            .iter()
            .map(|r| r.get_id() + ",")
            .filter(|id| id != "Invalid range")
            .collect::<String>();
        encode.push_str(&underline_ranges);
        encode.push_str("\n");
        // Encode bold
        encode.push_str("#b: ");
        let bold_ranges = self.bold_buffer
            .iter()
            .map(|r| r.get_id() + ",")
            .filter(|id| id != "Invalid range")
            .collect::<String>();
        encode.push_str(&bold_ranges);
        encode.push_str("\n");
        for (i, line) in (&self.lines).iter().enumerate() {
            encode.push_str(&line.buffer.clone().join(""));
            if i + 1 != self.lines.len() { encode.push('\n') }
        }
        fs::write(valid_filepath, &encode).expect(&format!("Unable to write file to {}", filepath));
        self.send_event(EditorEvent::LoadFile(filepath.into()))
    }

    /// Ask for the filepath to load
    pub fn load(&mut self) {
        let mut path_items = vec![];
        for (name, path) in self.get_recent_paths() {
            path_items.push(MenuItem::new(&name, MenuAction::OpenWithInput(path)));
        }
        let path_submenu = ContextualMenu::new_with_items(self.system_font.clone(), self.event_sender.clone().unwrap(), path_items);
        let mut menu_items = vec![
            MenuItem::new_with_submenu("Open ...", path_submenu),
            MenuItem::separator()
        ];
        for (name, path) in self.get_recent_files() {
            menu_items.push(MenuItem::new(&name, MenuAction::Open(path)));
        }
        self.menu.open_with(menu_items);
    }

    /// Load a specific path
    pub fn load_file(&mut self, filepath: &str) {
        if filepath.ends_with(".txt") { self.load_txt_file(filepath) }
        else if filepath.ends_with(".drn") { self.load_drn_file(filepath) }
        // TODO: .rtf ?
    }

    pub fn load_txt_file(&mut self, filepath: &str) {
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
        let mut i : usize = self.lines.len() - 1;
        while i > 0 && self.lines[i].is_empty() { // Remove the empty lines at the end of the file
            self.lines.pop();
            i -= 1;
        }
        self.cursor.move_to(0, 0);
        self.update_text_layout();
        if filepath != "new-file.txt" {
            self.add_to_recent_paths(filepath);
            self.add_to_recent_files(filepath);
        }
        self.send_event(EditorEvent::LoadFile(filepath.into()))
    }

    pub fn load_drn_file(&mut self, filepath: &str) {
        let valid_filepath = fs::canonicalize(filepath).expect("Invalid filepath");
        self.lines = vec![Line::new(Rc::clone(&self.font))];
        self.selection.reset();
        self.filepath = Some(filepath.into());
        let file_content = fs::read_to_string(&valid_filepath).expect(&format!("Unable to load file to {}", filepath));
        let content_lines = file_content.split('\n').collect();
        self.underline_buffer = Range::get_ranges_from_drn_line("#u:", &content_lines);
        self.bold_buffer = Range::get_ranges_from_drn_line("#b:", &content_lines);
        for (i, line) in content_lines[2..].iter().enumerate() {
            if i < self.lines.len() {
                self.lines.push(Line::new(Rc::clone(&self.font)));
            }
            self.lines[i].add_text(line);
        }
        let mut i : usize = self.lines.len() - 1;
        while i > 0 && self.lines[i].is_empty() { // Remove the empty lines at the end of the file
            self.lines.pop();
            i -= 1;
        }
        self.cursor.move_to(0, 0);
        self.update_text_layout();
        if filepath != "new-file.txt" {
            self.add_to_recent_paths(filepath);
            self.add_to_recent_files(filepath);
        }
        self.send_event(EditorEvent::LoadFile(filepath.into()))

    }

    pub fn get_animations(&mut self) -> Vec<&mut Option<Animation>> {
        let mut animations = vec![
            &mut self.cursor.animation.x, &mut self.cursor.animation.y,
            &mut self.camera.animation.x,  &mut self.camera.animation.y,
            &mut self.selection.start_animation.x, &mut self.selection.start_animation.y,
            &mut self.selection.end_animation.x, &mut self.selection.end_animation.y,
        ];
        for animation in self.menu.get_animations() {
            animations.push(animation)
        }
        animations
    }

    fn update_stats(&mut self) {
        if self.menu.is_visible && self.menu.get_focused_item().action == MenuAction::Information {
            self.menu.set_items(self.get_stats().iter().map(|s| {
                if s.starts_with("---") { return MenuItem::separator() }
                MenuItem::new(s, MenuAction::Information)
            }).collect());
        }
    }

    pub fn update_text_layout(&mut self) {
        let mut difference = 0;
        for (i, line) in (&mut self.lines).iter_mut().enumerate() {
            let diff = line.update_text_layout();
            if i as u32 == self.cursor.y { difference = diff; }
        }
        self.font.borrow_mut().style_changed = false;
        self.cursor.move_to((self.cursor.x as i32 - difference) as u32, self.cursor.y);
        self.update_stats();
    }

    pub fn update(&mut self, dt: f32) {
        let start_time = Instant::now();
        let animations = self.get_animations();
        for animation in animations {
            if let Some(anim) = animation {
                if !anim.has_started {
                    anim.start();
                }
                anim.update(dt);
                if anim.is_ended { *animation = Option::None; }
            }
        }
        self.stats.update_duration = start_time.elapsed();
    }

    pub fn render(&mut self, graphics: &mut Graphics2D) {
        let start_time = Instant::now();
        let char_width = self.font.borrow().char_width;
        let char_height = self.font.borrow().char_height;

        let mut previous_line_height = 0.;
        self.selection.render(&self.lines, &self.camera, graphics);
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
        let line_offset = self.get_current_line().alignment_offset;
        let line_camera = Camera::from_with_offset(&self.camera, Vector2::new(-line_offset, 0.));

        // draw underline
        for range in &mut self.underline_buffer {
            assert!(range.is_valid());
            let line = &self.lines[range.start.unwrap().y as usize];
            let line_offset = line.alignment_offset;
            let line_camera = Camera::from_with_offset(&self.camera, Vector2::new(-line_offset, 0.));
            let lines_index = range.get_lines_index(&self.lines);
            let initial_y = range.get_real_start().unwrap().y;
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
        let menu_position = self.cursor.position() - self.camera.position() + Vector2::new(CURSOR_OFFSET_X, self.font.borrow().char_height);
        self.menu.render(menu_position, graphics);
        graphics.draw_rectangle( // draw the title bar
            Rectangle::new(
                Vector2::new(0., 0.),
                Vector2::new(self.camera.width, EDITOR_OFFSET_TOP),
            ),
            Color::WHITE
        );
        // draw the title bar line
        if self.camera.computed_y() > self.padding + EDITOR_OFFSET_TOP {
            graphics.draw_line(
                Vector2::new(0., EDITOR_OFFSET_TOP),
                Vector2::new(self.camera.width, EDITOR_OFFSET_TOP),
                0.5,
                Color::GRAY
            );
        }
        self.stats.draw_duration = start_time.elapsed();
    }
}
