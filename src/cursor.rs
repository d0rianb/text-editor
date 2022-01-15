use std::cell::RefCell;
use std::rc::Rc;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::shape::Rectangle;
use speedy2d::Graphics2D;
use speedy2d::window::UserEventSender;

use crate::animation::{Animation, EasingFunction};
use crate::camera::Camera;
use crate::EditorEvent;
use crate::font::Font;

pub const CURSOR_WIDTH: f32 = 3.0;
pub const CURSOR_OFFSET_X: f32 = 2.0;

#[allow(dead_code)]
pub enum CursorType {
    Carret,
    Cross,
    // Rect,
    // Underscore,
}

pub struct Cursor {
    pub x: u32,
    pub y: u32,
    pub font: Rc<RefCell<Font>>,
    pub animation: Vector2<Option<Animation>>,
    pub cursor_type: CursorType,
    pub event_sender: Option<UserEventSender<EditorEvent>>,
}

impl Cursor {
    pub fn new(x: u32, y: u32, font: Rc<RefCell<Font>>) -> Self {
        Self {
            x,
            y,
            font,
            cursor_type: CursorType::Carret,
            animation: Vector2 { x: Option::None, y: Option::None },
            event_sender: Option::None,
        }
    }

    pub fn move_to(&mut self, x: u32, y: u32) {
        self.transition(x, y);
        self.x = x;
        self.y = y;
    }

    pub fn computed_x(&self) -> f32 {
        if let Some(animation) = &self.animation.x {
            animation.value
        } else {
            self.x as f32 * self.font.borrow().char_width
        }
    }

    pub fn computed_y(&self) -> f32 {
        if let Some(animation) = &self.animation.y {
            animation.value
        }
        else {
            self.y as f32 * self.font.borrow().char_height
        }
    }

    pub fn position(&self) -> Vector2<f32> {
        Vector2::new(self.computed_x(), self.computed_y())
    }

    fn transition(&mut self, x: u32, y: u32) {
        let start_x = if let Some(animation_x) = &self.animation.x { animation_x.value } else { self.computed_x() };
        let start_y = if let Some(animation_y) = &self.animation.y { animation_y.value } else { self.computed_y() };
        let duration = 100.;
        let es = self.event_sender.clone().unwrap();
        let new_animation_x = Animation::new(start_x, x as f32 * self.font.borrow().char_width, duration, EasingFunction::SmootherStep, es.clone());
        let new_animation_y = Animation::new(start_y, y as f32 * self.font.borrow().char_height, duration, EasingFunction::SmootherStep, es);
        self.animation.x = Some(new_animation_x);
        self.animation.y = Some(new_animation_y);
    }

    fn get_carret_rectangle(&self, camera: &Camera) -> Rectangle<f32> {
        let x = self.computed_x() - camera.computed_x();
        let y = self.computed_y() - camera.computed_y();
        Rectangle::new(
            Vector2::new(x + CURSOR_OFFSET_X, y),
            Vector2::new(
                (x + CURSOR_OFFSET_X + CURSOR_WIDTH) as f32,
                y + self.font.borrow().char_height,
            ),
        )
    }

    pub fn render(&self, camera: &Camera, graphics: &mut Graphics2D) {
        match self.cursor_type {
            CursorType::Carret => graphics.draw_rectangle(self.get_carret_rectangle(camera), Color::BLACK),
            CursorType::Cross => {
                let x = -camera.computed_x() + self.computed_x() + self.font.borrow().char_width /2.;
                let y = -camera.computed_y() + self.computed_y() + self.font.borrow().char_height /2.;
                graphics.draw_line(Vector2::new(x, 0.),Vector2::new(x, self.font.borrow().editor_size.y - camera.computed_y()), CURSOR_WIDTH/5., Color::BLACK);
                graphics.draw_line(Vector2::new(0., y),Vector2::new(self.font.borrow().editor_size.x - camera.computed_x(), y), CURSOR_WIDTH/5., Color::BLACK);
            },
        }
    }
}
