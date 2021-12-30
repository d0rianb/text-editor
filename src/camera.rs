use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::Graphics2D;
use speedy2d::window::UserEventSender;

use crate::animation::{Animation, EasingFunction};
use crate::cursor::Cursor;
use crate::editor::{EDITOR_OFFSET_TOP, EDITOR_PADDING};
use crate::EditorEvent;

pub(crate) struct Camera {
    x: f32,
    y: f32,
    pub width: f32,
    pub height: f32,
    pub safe_zone_size: f32,
    pub animation: Vector2<Option<Animation>>,
    pub event_sender: Option<UserEventSender<EditorEvent>>,
}

const INITIAL_X: f32 = -EDITOR_PADDING;
const INITIAL_Y: f32 = -EDITOR_OFFSET_TOP - EDITOR_PADDING;

impl Camera {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            x: 0.,
            y: 0.,
            width,
            height,
            safe_zone_size: 30.0,
            animation: Vector2 { x: Option::None, y: Option::None },
            event_sender: Option::None
        }
    }

    pub fn move_x(&mut self, dx: f32) {
        let new_x = (self.x + dx).clamp(0., self.width);
        self.transition(new_x + INITIAL_X, self.y + INITIAL_Y);
        self.x = new_x;
    }

    pub fn move_y(&mut self, dy: f32) {
        let new_y = (self.y + dy).clamp(0., self.height);
        self.transition(self.x + INITIAL_X, new_y + INITIAL_Y);
        self.y = new_y;
    }

    pub fn reset(&mut self) {
        self.x = 0.;
        self.y = 0.;
    }

    pub fn computed_x(&self) -> f32 {
        if let Some(animation) = &self.animation.x {
            animation.value
        } else {
            self.x + INITIAL_X
        }
    }

    pub fn computed_y(&self) -> f32 {
        if let Some(animation) = &self.animation.y {
            animation.value
        }
        else {
            self.y + INITIAL_Y
        }
    }

    pub fn get_cursor_real_y(&self, cursor: &Cursor) -> f32 {
        cursor.computed_y() + INITIAL_Y - cursor.font.borrow().char_height
    }

    fn transition(&mut self, x: f32, y: f32) {
        let start_x = if let Some(animation_x) = &self.animation.x { animation_x.value } else { self.computed_x() };
        let start_y = if let Some(animation_y) = &self.animation.y { animation_y.value } else { self.computed_y() };
        let duration = 100.;
        let es = self.event_sender.clone();
        let new_animation_x = Animation::new(start_x, x, duration, EasingFunction::SmootherStep, es.clone());
        let new_animation_y = Animation::new(start_y, y, duration, EasingFunction::SmootherStep, es);
        self.animation.x = Some(new_animation_x);
        self.animation.y = Some(new_animation_y);
    }

    pub fn _render(&self, graphics: &mut Graphics2D) {
        graphics.draw_line(
            Vector2::new(-INITIAL_X + self.safe_zone_size, -INITIAL_Y +  self.safe_zone_size),
            Vector2::new(-INITIAL_X + self.width - self.safe_zone_size, -INITIAL_Y +  self.safe_zone_size),
            1.,
            Color::BLACK
        );
        graphics.draw_line(
            Vector2::new(-INITIAL_X + self.safe_zone_size, -INITIAL_Y +  self.height - self.safe_zone_size),
            Vector2::new(-INITIAL_X + self.width -  self.safe_zone_size, -INITIAL_Y +  self.height - self.safe_zone_size),
            1.,
            Color::BLACK
        );
    }
}