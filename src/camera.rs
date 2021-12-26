use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::Graphics2D;
use speedy2d::window::UserEventSender;

use crate::animation::{Animation, EasingFunction};
use crate::EditorEvent;

pub(crate) struct Camera {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub safe_zone_size: f32,
    pub animation: Vector2<Option<Animation>>,
    pub event_sender: Option<UserEventSender<EditorEvent>>,
}

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
        self.transition(new_x, self.y);
        self.x = new_x;
    }

    pub fn move_y(&mut self, dy: f32) {
        let new_y = (self.y + dy).clamp(0., self.height);
        self.transition(self.x, new_y);
        self.y = new_y;
    }

    pub fn computed_x(&self) -> f32 {
        if let Some(animation) = &self.animation.x {
            animation.value
        } else {
            self.x
        }
    }

    pub fn computed_y(&self) -> f32 {
        if let Some(animation) = &self.animation.y {
            animation.value
        }
        else {
            self.y
        }
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

    pub fn render(&self, graphics: &mut Graphics2D) {
        graphics.draw_line(
            Vector2::new(self.safe_zone_size, self.safe_zone_size),
            Vector2::new(self.width - self.safe_zone_size, self.safe_zone_size),
            1.,
            Color::BLACK
        );
        graphics.draw_line(
            Vector2::new(self.safe_zone_size, self.height - self.safe_zone_size),
            Vector2::new(self.width -  self.safe_zone_size, self.height - self.safe_zone_size),
            1.,
            Color::BLACK
        );
    }
}