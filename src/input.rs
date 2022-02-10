use std::cmp::Ordering;
use std::fs;
use std::rc::Rc;
use lazy_static::lazy_static;
use regex::Regex;
use itertools::Itertools;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::font::{FormattedTextBlock, TextOptions};
use speedy2d::Graphics2D;
use speedy2d::shape::Rectangle;
use speedy2d::window::{UserEventSender, VirtualKeyCode};

use crate::{Animation, Editable, Editor, EditorEvent, FocusElement, MenuId};
use crate::menu_actions::{MenuAction, MenuActionFn};
use crate::animation::EasingFunction;
use crate::camera::Camera;
use crate::render_helper::draw_rounded_rectangle_with_border;

pub const MIN_INPUT_WIDTH: f32 = 200.;
pub const MAX_INPUT_WIDTH: f32 = 500.;

const ANIMATION_DURATION: f32 = 100.;

#[allow(unused)]
#[derive(PartialEq)]
pub enum Validator {
    File,
    Path,
    None
}

pub struct Input {
    pub editor: Editor,
    pub menu_id: MenuId,
    is_focus: bool,
    action_fn: MenuActionFn,
    width: f32,
    height: f32,
    suggestion: String,
    suggestion_offset: i32,
    suggestion_test_layout: Rc<FormattedTextBlock>,
    validator: Validator,
    animation_width: Option<Animation>,
    intermediate_result: bool,
    has_error: bool,
}

impl Editable for Input {
    fn add_char(&mut self, c: String) { self.editor.add_char(c); self.on_insert(); self.set_suggestion(); }

    fn delete_char(&mut self) { self.editor.delete_char(); self.on_insert(); self.set_suggestion(); }

    fn handle_key(&mut self, keycode: VirtualKeyCode) {
        match keycode {
            VirtualKeyCode::Right => self.move_cursor_relative(1, 0),
            VirtualKeyCode::Left => self.move_cursor_relative(-1, 0),
            VirtualKeyCode::Up => {},   // Move to other menu fields
            VirtualKeyCode::Down => {}, // Move to other menu fields
            VirtualKeyCode::Backspace => self.delete_char(),
            VirtualKeyCode::Delete => { self.move_cursor_relative(1, 0); self.delete_char(); },
            VirtualKeyCode::Return => self.submit(),
            VirtualKeyCode::Escape => self.unfocus(),
            VirtualKeyCode::Tab => if self.editor.modifiers.shift() { self.suggestion_offset -= 1 } else { self.suggestion_offset += 1 },
            _ => return self.editor.handle_key(keycode)
        }
        self.set_suggestion();
        self.update_text_layout();
    }

    fn move_cursor(&mut self, position: Vector2<u32>) { self.editor.move_cursor(position) }

    fn move_cursor_relative(&mut self, rel_x: i32, _rel_y: i32) {
        if self.editor.cursor.x as i32 + rel_x < 0 { return self.unfocus(); }
        let line = &mut self.editor.lines[0];
        if self.editor.cursor.x >= line.buffer.len() as u32 && rel_x > 0 && self.validator != Validator::None {
            line.add_text(&self.suggestion);
            self.editor.move_cursor_relative(self.suggestion.len() as i32, 0);
            return;
        }
        self.editor.move_cursor_relative(rel_x, 0)
    }

    fn shortcut(&mut self, c: char) {
        match c {
            'c' => self.copy(),
            'v' => self.paste(),
            'x' => { self.copy(); self.delete_selection() },
            'a' => self.select_all(),
            'l' => self.select_current_line(),
            'L' => { self.select_current_line(); self.delete_selection() },
            'd' => self.select_current_word(),
            'D' => { self.select_current_word(); self.delete_selection() },
            'g' => self.on_insert(),
            _ => {}
        }
    }

    fn begin_selection(&mut self) { self.editor.begin_selection() }

    fn end_selection(&mut self) { self.editor.end_selection() }

    fn update_selection(&mut self, position: Vector2<f32>) { self.editor.update_selection(position) }

    fn delete_selection(&mut self) { self.editor.delete_selection() }

    fn get_mouse_position_index(&mut self, position: Vector2<f32>) -> Vector2<u32> { self.editor.get_mouse_position_index(position) }

    fn get_valid_cursor_position(&mut self, position: Vector2<u32>) -> Vector2<u32> { self.editor.get_valid_cursor_position(position) }

    fn select_current_word(&mut self) { self.editor.select_current_word() }

    fn select_all(&mut self) { self.editor.select_all() }

    fn select_current_line(&mut self) { self.editor.select_current_line() }

    fn copy(&mut self) { self.editor.copy() }

    fn paste(&mut self) { self.editor.paste() }
}

impl Input {
    pub fn new(menu_id: MenuId, action_fn: MenuActionFn, es: UserEventSender<EditorEvent>) -> Self {
        let mut editor = Editor::new(MIN_INPUT_WIDTH, 50., Vector2::ZERO, 10.); // arbitrary
        editor.font.borrow_mut().change_font_size(-6); // Set font size to 10
        let offset = Vector2::new(0., (50. - editor.font.borrow().char_height) / 2. - 10.);
        editor.set_offset(offset);
        editor.set_event_sender(Some(es));
        editor.camera.safe_zone_size = 0.;
        let blank_text_layout = editor.lines[0].formatted_text_block.clone();
        Self {
            editor,
            is_focus: false,
            menu_id,
            action_fn,
            width: 0.,
            height: 50.,
            suggestion: "".into(),
            suggestion_offset: 0,
            suggestion_test_layout: blank_text_layout,
            validator: Validator::None,
            animation_width: Option::None,
            intermediate_result: false,
            has_error: false
        }
    }

    pub fn set_intermediate_result(&mut self) { self.intermediate_result = true; }

    pub fn focus(&mut self) {
        if self.width == 0. { self.set_width(MIN_INPUT_WIDTH); }
        self.is_focus = true;
        self.editor.update_camera();
        self.set_suggestion();
        self.editor.event_sender.as_ref().unwrap().send_event(
            EditorEvent::Focus(FocusElement::MenuInput(self.menu_id))
        ).unwrap()
    }

    pub fn unfocus(&mut self) {
        self.set_width(0.);
        self.is_focus = false;
        self.editor.event_sender.as_ref().unwrap().send_event(
            EditorEvent::Focus(FocusElement::Menu(self.menu_id))
        ).unwrap()
    }

    fn set_width(&mut self, width: f32) {
        let animation_width = Animation::new(self.computed_width(), width, ANIMATION_DURATION, EasingFunction::SmootherStep, self.editor.event_sender.clone().unwrap());
        self.animation_width = Some(animation_width);
        self.width = width;
        self.editor.camera.width = width;
    }

    pub fn set_placeholder(&mut self, text: &str) {
        let line = self.editor.lines.get_mut(0).unwrap();
        line.empty();
        line.add_text(text);
        self.move_cursor(Vector2::new(text.len() as u32, 0));
        self.update_text_layout();
        self.editor.camera.reset();
    }

    pub fn set_validator(&mut self, validator: Validator) {
        self.validator = validator;
    }

    fn get_sorted_suggestion_items(&self, input: &str) -> Vec<String> {
        fs::read_dir(input)
            .expect("Unable to access the sub directories")
            .map(|dir_entry| {
                let path_buf =  dir_entry.unwrap().path();
                let mut name =  path_buf.file_name().unwrap().to_os_string();
                if path_buf.is_dir() { name.push("/") }
                name.to_str().unwrap().to_string()
            })
            .sorted_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()))
            .sorted_by(|a, b| {
                if b.starts_with('.') {
                    if a.starts_with('.') { return a.to_lowercase().cmp(&b.to_lowercase()); };
                    return Ordering::Less;
                }
                return Ordering::Greater;
            })
            .collect()
    }

    fn set_suggestion(&mut self) {
        lazy_static! {
            static ref PATH_REGEX: Regex = Regex::new(r"^.*/").unwrap();
            static ref LAST_WORD_REGEX: Regex = Regex::new(r#"/([\w.\-\\\\ ]*)$"#).unwrap();
        }
        let input = self.editor.lines[0].get_text();
        let path_groups = PATH_REGEX.captures(&input);
        let last_word_groups = LAST_WORD_REGEX.captures(&input);
        let path: &str = if path_groups.is_some() { path_groups.unwrap().get(0).map_or("", |m| m.as_str()) } else { "/" };
        let last_word: &str = if last_word_groups.is_some() { last_word_groups.unwrap().get(1).map_or("", |m| m.as_str()) } else { "" };
        let sorted_suggestions = self.get_sorted_suggestion_items(&path);
        if sorted_suggestions.len() == 0  { self.suggestion = String::new() }
        let nb_suggestions = sorted_suggestions.len();
        if last_word == "" {
            if self.suggestion_offset < 0 { self.suggestion_offset += nb_suggestions as i32 }
            self.suggestion = sorted_suggestions[self.suggestion_offset as usize % nb_suggestions].to_string();
            return;
        }
        let mut guess = String::new();
        for i in 0 .. nb_suggestions {
            let name = &sorted_suggestions[i] as &str;
            if (&name.to_lowercase() as &str).cmp(&last_word.to_lowercase()).is_ge() && name.to_lowercase().starts_with(&last_word.to_lowercase()) {
                let input_len = last_word.len();
                if input_len <= name.len() {
                    guess = name[input_len..].to_string();
                    break;
                }
            }
        }
        self.suggestion = guess;
    }

    fn validate(&self, text: &str) -> bool {
        lazy_static! { static ref FILE_REGEX: Regex = Regex::new(r".txt$").unwrap(); }
        match self.validator {
            Validator::File => FILE_REGEX.is_match(text),
            Validator::None => true,
            _ => false,
        }
    }

    fn on_insert(&mut self) {
        if self.has_error { self.has_error = false; }
        if !self.intermediate_result { return; }
        let result = self.editor.lines.first().unwrap().get_text();
        self.editor.event_sender.as_ref().unwrap().send_event(
            EditorEvent::MenuItemSelected((self.action_fn)(result))
        ).unwrap();
    }

    fn submit(&mut self) {
        let result = self.editor.lines.first().unwrap().get_text();
        if !self.validate(&result) {
            self.has_error = true;
            return;
        }
        self.editor.event_sender.as_ref().unwrap().send_event(
            EditorEvent::MenuItemSelected((self.action_fn)(result))
        ).unwrap();
        self.unfocus();
        self.editor.event_sender.as_ref().unwrap().send_event(
            EditorEvent::MenuItemSelected(MenuAction::CloseMenu)
        ).unwrap();
    }

    fn computed_width(&self) -> f32 {
        if let Some(animation) = &self.animation_width { animation.value } else { self.width }
    }

    pub fn get_animations(&mut self) -> Vec<&mut Option<Animation>> {
        let mut animations = vec![&mut self.animation_width];
        animations.append(&mut self.editor.get_animations());
        animations
    }

    pub fn update_text_layout(&mut self) {
        self.editor.update_text_layout();
        self.suggestion_test_layout = self.editor.font.borrow().layout_text(&self.suggestion, TextOptions::default());
        let width_left = self.width - self.editor.lines.first().unwrap().formatted_text_block.width() - self.suggestion_test_layout.width();
        const WIDTH_OFFSET: f32 = 2.;
        if width_left < WIDTH_OFFSET {
            self.set_width((self.width + WIDTH_OFFSET - width_left).clamp(MIN_INPUT_WIDTH, MAX_INPUT_WIDTH));
        } else if width_left >= WIDTH_OFFSET {
            self.set_width((self.width - width_left + WIDTH_OFFSET).clamp(MIN_INPUT_WIDTH, MAX_INPUT_WIDTH));
        }
    }

    pub fn render(&mut self, x: f32, y: f32, graphics: &mut Graphics2D) {
        if !self.is_focus { return; }
        lazy_static! {
            static ref BG_COLOR: Color = Color::from_int_rgb(250, 250, 250);
            static ref BORDER_COLOR: Color = Color::from_int_rgb(150, 150, 150);
            static ref ERROR_BORDER_COLOR: Color = Color::from_int_rgb(235, 20, 20);
        }

        // Draw background
        let border_color: Color = if self.has_error { *ERROR_BORDER_COLOR } else { *BORDER_COLOR };
        draw_rounded_rectangle_with_border(x, y, self.computed_width(), self.height, 8., 0.5, *BG_COLOR, border_color, graphics);
        // Draw text
        let line = self.editor.lines.first().unwrap();
        let input_camera = Camera::from_with_offset(&self.editor.camera, Vector2::new(-x, -y));
        self.editor.selection.render(&self.editor.lines, &input_camera, graphics);
        graphics.set_clip(Some(
            Rectangle::new(
                Vector2::new(x as i32, y as i32),
                Vector2::new((x + self.width) as i32, (y + self.height) as i32)
            )
        ));
        line.render(x - self.editor.camera.computed_x(), y - self.editor.camera.computed_y(), graphics);
        graphics.draw_text(
            Vector2::new(x - self.editor.camera.computed_x() + line.formatted_text_block.width(), y - self.editor.camera.computed_y()),
            Color::GRAY,
            &self.suggestion_test_layout
        );
        graphics.set_clip(Option::None);
        self.editor.cursor.render(&input_camera, graphics);
    }
}