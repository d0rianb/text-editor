use std::rc::Rc;
use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::Graphics2D;
use speedy2d::window::{UserEventSender, VirtualKeyCode};
use crate::{Animation, Editable, Editor, EditorEvent, FocusElement, MenuAction, MenuActionFn, MenuId};
use crate::animation::EasingFunction;
use crate::camera::Camera;
use crate::render_helper::{draw_rect_border, draw_rounded_rectangle, draw_rounded_rectangle_with_border};

pub const MIN_INPUT_WIDTH: f32 = 200.;

const ANIMATION_DURATION: f32 = 100.;

pub struct Input {
    pub(crate) editor: Editor,
    menu_id: MenuId,
    action_fn: MenuActionFn,
    width: f32,
    height: f32,
    animation_width: Option<Animation>
}

impl Editable for Input {
    fn add_char(&mut self, c: String) { self.editor.add_char(c) }

    fn delete_char(&mut self) { self.editor.delete_char() }

    fn handle_key(&mut self, keycode: VirtualKeyCode) {
        match keycode {
            VirtualKeyCode::Right => self.move_cursor_relative(1, 0),
            VirtualKeyCode::Left => self.move_cursor_relative(-1, 0),
            VirtualKeyCode::Up => self.move_cursor_relative(0, -1),
            VirtualKeyCode::Down => self.move_cursor_relative(0, 1),
            VirtualKeyCode::Backspace => self.delete_char(),
            VirtualKeyCode::Delete => { self.move_cursor_relative(1, 0); self.delete_char(); },
            VirtualKeyCode::Return => self.submit(),
            VirtualKeyCode::Escape => self.unfocus(),
            VirtualKeyCode::Tab => {},
            _ => return self.editor.handle_key(keycode)
        }
        self.update_text_layout();
    }

    fn move_cursor(&mut self, position: Vector2<u32>) { self.editor.move_cursor(position) }

    fn move_cursor_relative(&mut self, rel_x: i32, rel_y: i32) { self.editor.move_cursor_relative(rel_x, rel_y) }

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
        editor.set_event_sender(Some(es));
        let offset = Vector2::new(0., (50. - editor.font.borrow().char_height) / 2. - 10.);
        editor.set_offset(offset);
        Self {
            editor,
            menu_id,
            action_fn,
            width: MIN_INPUT_WIDTH,
            height: 50.,
            animation_width: Option::None
        }
    }

    pub fn focus(&self) {
        let es = self.editor.event_sender.as_ref().unwrap();
        es.send_event(EditorEvent::Focus(FocusElement::MenuInput(self.menu_id))).unwrap()
    }

    pub fn unfocus(&self) {
        let es = self.editor.event_sender.as_ref().unwrap();
        es.send_event(EditorEvent::Focus(FocusElement::Menu(self.menu_id))).unwrap()
    }

    fn set_width(&mut self, width: f32) {
        let animation_width = Animation::new(self.computed_width(), width, ANIMATION_DURATION, EasingFunction::SmootherStep, self.editor.event_sender.clone().unwrap());
        self.animation_width = Some(animation_width);
        self.width = width;
    }

    fn submit(&self) {
        let result = self.editor.lines.first().unwrap().get_text();
        self.editor.event_sender.as_ref().unwrap().send_event(
            EditorEvent::MenuItemSelected((self.action_fn)(result))
        );
        // TODO: Check if it's valid
        self.close()
    }

    fn computed_width(&self) -> f32 {
        if let Some(animation) = &self.animation_width { animation.value } else { self.width }
    }

    pub fn get_animation(&mut self) -> Vec<&mut Option<Animation>> {
        let mut animations = vec![&mut self.animation_width];
        animations.append(&mut self.editor.get_animations());
        animations
    }

    pub fn update_text_layout(&mut self) {
        self.editor.update_text_layout();
        let width_left = self.width - self.editor.lines.first().unwrap().formatted_text_block.width();
        const WIDTH_OFFSET: f32 = 25.;
        if width_left < WIDTH_OFFSET {
            self.set_width(self.width + width_left.abs() + WIDTH_OFFSET);
        } else if width_left >= 1.5 * WIDTH_OFFSET {
            self.set_width((self.width - WIDTH_OFFSET).max(MIN_INPUT_WIDTH));
        }
    }

    pub fn render(&mut self, x: f32, y: f32, graphics: &mut Graphics2D) {
        // Draw background
        draw_rounded_rectangle_with_border(x, y, self.computed_width(), self.height, 8., 0.5, Color::from_int_rgba(250, 250, 250, 255), graphics);
        // Draw text
        let line = self.editor.lines.first().unwrap();
        line.render(x - self.editor.camera.computed_x(), y - self.editor.camera.computed_y(), graphics);

        let input_camera = Camera::from_with_offset(&self.editor.camera, Vector2::new(-x, -y));
        self.editor.cursor.render(&input_camera, graphics);
        self.editor.selection.render(Rc::clone(&self.editor.font), &self.editor.lines, &input_camera, graphics);
    }
}