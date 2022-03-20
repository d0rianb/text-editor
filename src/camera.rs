use std::fmt::{Debug, Formatter};
use ifmt::iwrite;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::Graphics2D;
use speedy2d::window::UserEventSender;

use crate::animation::{Animation, EasingFunction};
use crate::cursor::Cursor;
use crate::EditorEvent;

#[derive(Clone)]
pub struct Camera {
    x: f32,
    y: f32,
    pub width: f32,
    pub height: f32,
    pub initial_x: f32,
    pub initial_y: f32,
    pub safe_zone_size: f32,
    pub animation: Vector2<Option<Animation>>,
    pub event_sender: Option<UserEventSender<EditorEvent>>,
}

impl Debug for Camera {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        iwrite!(f, "Camera : x: {self.computed_x()} | y: {self.computed_y()} \
         | initial_x: {self.initial_x} | initial_y: {self.initial_y} \
         | width: {self.width} | height: {self.height}")
    }
}


impl Camera {
    pub fn new(width: f32, height: f32, offset: Vector2<f32>, padding: f32) -> Self {
        Self {
            x: 0.,
            y: 0.,
            width: width - 2. * padding - offset.x,
            height: height - 2. * padding - offset.y,
            initial_x: -padding - offset.x,
            initial_y: -padding - offset.y,
            safe_zone_size: 30.0,
            animation: Vector2::new(Option::None, Option::None),
            event_sender: Option::None
        }
    }

    pub fn _from_real_origin(width: f32, height: f32) -> Self {
        Self {
            x: 0.,
            y: 0.,
            width,
            height,
            initial_x: 0.,
            initial_y: 0.,
            safe_zone_size: 0.0,
            animation: Vector2 { x: Option::None, y: Option::None },
            event_sender: Option::None
        }
    }

    pub fn from_with_offset(camera: &Self, offset: Vector2<f32>) -> Self {
        Self {
            x: camera.x + offset.x,
            y: camera.y + offset.y,
            width: camera.width,
            height: camera.height,
            initial_x: camera.initial_x,
            initial_y: camera.initial_y,
            safe_zone_size: 30.0,
            animation: Vector2::new(Option::None, Option::None),
            event_sender: camera.event_sender.clone()
        }
    }

    pub fn reset(&mut self) {
        self.x =  0.;
        self.y = 0.;
    }

    pub fn on_resize(&mut self, size: Vector2<u32>) {
        self.width = size.x as f32;
        self.height = size.y as f32;
    }

    pub fn move_x(&mut self, dx: f32) {
        let new_x = (self.x + dx).max(0.);
        self.transition(new_x + self.initial_x, self.y + self.initial_y);
        self.x = new_x;
    }

    pub fn move_y(&mut self, dy: f32) {
        let new_y = (self.y + dy).max(0.);
        self.transition(self.x + self.initial_x, new_y + self.initial_y);
        self.y = new_y;
    }

    pub fn computed_x(&self) -> f32 {
        if let Some(animation) = &self.animation.x { animation.value } else { self.x + self.initial_x }
    }

    pub fn computed_y(&self) -> f32 {
        if let Some(animation) = &self.animation.y { animation.value } else { self.y + self.initial_y }
    }

    pub fn position(&self) -> Vector2<f32> {
        Vector2::new(self.computed_x(), self.computed_y())
    }

    pub fn get_cursor_x_with_offset(&self, cursor: &Cursor) -> f32 {
        cursor.real_x() + self.initial_x - cursor.font.borrow().char_width
    }

    pub fn get_cursor_y_with_offset(&self, cursor: &Cursor) -> f32 {
        cursor.real_y() + self.initial_y - cursor.font.borrow().char_height
    }

    fn transition(&mut self, x: f32, y: f32) {
        let start_x = if let Some(animation_x) = &self.animation.x { animation_x.value } else { self.computed_x() };
        let start_y = if let Some(animation_y) = &self.animation.y { animation_y.value } else { self.computed_y() };
        let duration = 100.;
        let es = self.event_sender.clone().unwrap();
        let new_animation_x = Animation::new(start_x, x, duration, EasingFunction::SmootherStep, es.clone());
        let new_animation_y = Animation::new(start_y, y, duration, EasingFunction::SmootherStep, es);
        self.animation.x = Some(new_animation_x);
        self.animation.y = Some(new_animation_y);
    }

    pub fn _render(&self, graphics: &mut Graphics2D) {
        graphics.draw_line(
            Vector2::new(-self.initial_x + self.safe_zone_size, -self.initial_y +  self.safe_zone_size),
            Vector2::new(-self.initial_x + self.width - self.safe_zone_size, -self.initial_y +  self.safe_zone_size),
            1.,
            Color::BLACK
        );
        graphics.draw_line(
            Vector2::new(-self.initial_x + self.safe_zone_size, -self.initial_y +  self.height - self.safe_zone_size),
            Vector2::new(-self.initial_x + self.width -  self.safe_zone_size, -self.initial_y +  self.height - self.safe_zone_size),
            1.,
            Color::BLACK
        );
    }
}
