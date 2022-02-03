use std::cell::RefCell;
use std::rc::Rc;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::Graphics2D;
use speedy2d::shape::Rectangle;
use speedy2d::window::UserEventSender;

use crate::{Animation, EditorEvent};
use crate::animation::EasingFunction;
use crate::camera::Camera;
use crate::font::Font;
use crate::line::Line;
use crate::range::{get_line_length, Range};

const ANIMATION_DURATION: f32 = 100.; // ms

pub struct Selection {
    range: Range,
    font: Rc<RefCell<Font>>,
    pub event_sender: Option<UserEventSender<EditorEvent>>,
    pub start_animation: Vector2<Option<Animation>>,
    pub end_animation: Vector2<Option<Animation>>,
}

impl Clone for Selection {
    fn clone(&self) -> Self {
        Self {
            range: self.range,
            font: self.font.clone(),
            event_sender: self.event_sender.clone(),
            start_animation: Vector2::new(None, None),
            end_animation: Vector2::new(None, None),
        }
    }
}

impl Selection {
    pub fn new(font: Rc<RefCell<Font>>) -> Self {
       Self {
           range: Default::default(),
           font,
           event_sender: Option::None,
           start_animation: Vector2::new(None, None),
           end_animation: Vector2::new(None, None),
       }
    }

    pub fn start(&self) -> Option<Vector2<u32>> {
        self.range.get_real_start()
    }

    pub fn end(&self) -> Option<Vector2<u32>> {
        self.range.get_real_end()
    }

    pub fn set_start(&mut self, position: Vector2<u32>) {
        let start = self.range.start;
        let char_width =  self.font.borrow().char_width;
        let char_height =  self.font.borrow().char_height;
        let es = self.event_sender.clone().unwrap();
        if let Some(start) = start {
            self.start_animation = Vector2::new(
                Some(Animation::new(start.x as f32 * char_width, position.x as f32 * char_width, ANIMATION_DURATION, EasingFunction::SmootherStep, es.clone())),
                Some(Animation::new(start.y as f32 * char_height, position.y as f32 * char_height, ANIMATION_DURATION, EasingFunction::SmootherStep, es.clone()))
            );
        }
        self.range.start(position)
    }

    pub fn set_end(&mut self, position: Vector2<u32>) {
        let end = self.range.end;
        let char_width =  self.font.borrow().char_width;
        let char_height =  self.font.borrow().char_height;
        let es = self.event_sender.clone().unwrap();
        if let Some(end) = end {
            self.end_animation = Vector2::new(
                Some(Animation::new(end.x as f32 * char_width, position.x as f32 * char_width, ANIMATION_DURATION, EasingFunction::SmootherStep, es.clone())),
                Some(Animation::new(end.y as f32 * char_height, position.y as f32 * char_height, ANIMATION_DURATION, EasingFunction::SmootherStep, es.clone()))
            );
        }
        self.range.end(position)
    }

    pub fn set(&mut self, start: Vector2<u32>, end: Vector2<u32>) {
        self.set_start(start);
        self.set_end(end);
    }

    pub fn is_valid(&self) -> bool {
        self.range.is_valid()
    }

    pub fn add(&mut self, range: Range) {
        self.range.add(range)
    }

    pub fn reset(&mut self) {
        self.range.reset();
        self.start_animation = Vector2::new(None, None);
        self.end_animation = Vector2::new(None, None);
    }

    pub fn get_range(&self) -> Range {
        self.range
    }

    fn computed_start(&self) -> Vector2<f32> {
        assert!(self.is_valid());
        let start = self.start().unwrap();
        let animation = if start == self.range.start.unwrap() { &self.start_animation } else { &self.end_animation };
        let x = if let Some(animation) = &animation.x { animation.value } else { start.x as f32 * self.font.borrow().char_width };
        let y = if let Some(animation) = &animation.y { animation.value } else { start.y as f32 * self.font.borrow().char_height };
        Vector2::new(x, y)
    }

    fn computed_end(&self) -> Vector2<f32> {
        assert!(self.is_valid());
        let end = self.end().unwrap();
        let animation = if end == self.range.end.unwrap() { &self.end_animation } else { &self.start_animation };
        let x = if let Some(animation) = &animation.x { animation.value } else { end.x as f32 * self.font.borrow().char_width };
        let y = if let Some(animation) = &animation.y { animation.value } else { end.y as f32 * self.font.borrow().char_height };
        Vector2::new(x, y)
    }

    pub fn get_lines_index(&mut self, lines: &[Line]) -> Vec<(u32, u32)> {
        self.range.get_lines_index(lines)
    }

    fn get_lines_bounds(&mut self, lines: &[Line]) -> Vec<(f32, f32)> {
        if !self.is_valid() { return vec![]; }
        let font_height = self.font.borrow().char_height;
        let start = self.start().unwrap();
        let end = self.end().unwrap();
        let mut result = vec![];
        for y in start.y..=end.y {
            if y == start.y {
                if start.y == end.y {
                    result.push((self.computed_start().x, self.computed_end().x)) } else { result.push((self.computed_start().x, get_line_length(y, lines) as f32 * font_height))
                }
            } else if y == end.y { result.push((0., self.computed_end().x)) } else { result.push((0., get_line_length(y, lines) as f32 * font_height)) }
        }
        result
    }

    pub fn render(&mut self, lines: &[Line], camera: &Camera, graphics: &mut Graphics2D) {
        if !self.is_valid() { return; }
        let font_height = self.font.borrow().char_height;
        let initial_y = self.start().unwrap().y as f32 * font_height - camera.computed_y();
        let lines_bounds = self.get_lines_bounds(lines);
        for (i, bounds) in lines_bounds.iter().enumerate() { // TODO: cache ?
            let mut line_y = initial_y + i as f32 * font_height;
            let line = &lines[self.start().unwrap().y as usize + i];
            let line_offset = line.alignment_offset;
            let line_camera = Camera::from_with_offset(camera, Vector2::new(-line_offset, 0.));
            if i == 0 { line_y = self.computed_start().y - camera.computed_y() }
            if i + 1 == lines_bounds.len() { line_y = self.computed_end().y - camera.computed_y() }
            graphics.draw_rectangle(
                Rectangle::new(
                    Vector2::new(bounds.0 - line_camera.computed_x(), line_y),
                    Vector2::new(bounds.1 - line_camera.computed_x(), line_y + font_height),
                ),
                Color::from_int_rgba(235, 235, 235, 255),
            )
        }
    }
}