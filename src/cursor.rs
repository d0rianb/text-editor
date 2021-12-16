use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::Graphics2D;
use speedy2d::shape::Rectangle;

use std::cmp;

use crate::font::Font;

const EDITOR_PADDING: u32 = 5;
const CURSOR_WIDTH: f32 = 3.0;
const CURSOR_OFFSET_X: f32 = 5.0;
const OFFSET: Vector2<u32> = Vector2 { x: EDITOR_PADDING, y: EDITOR_PADDING };

pub fn _clamp<T: Ord>(min: T, x: T, max: T) -> T {
    return cmp::max(min, cmp::min(x, max));
}

pub(crate) struct Cursor {
    pub x: u32,
    pub y: u32,
    pub font: Font,
}

impl Cursor {
    pub fn move_relative(&mut self, rel_x: i32, rel_y: i32) {
        self.x = cmp::max(0, self.x as i32 + rel_x) as u32;
        self.y = cmp::max(0, self.y as i32 + rel_y) as u32;
    }

    pub fn move_to(&mut self, x: u32, y: u32) {
        self.x = (x as f32 / self.font.char_width) as u32 + OFFSET.x;
        self.y = (y as f32 / self.font.char_height) as u32 + OFFSET.y;
        // transition(x, y);
    }

    pub fn computed_x(&self) -> f32 { self.x as f32 * self.font.char_width }
    pub fn computed_y(&self) -> f32 { self.y as f32 * self.font.char_height }

    fn _transition(&mut self, x: u32, y: u32) {}

    fn get_rectangle(&self) -> Rectangle<f32> {
        Rectangle::new(
            Vector2::new(self.computed_x() + CURSOR_OFFSET_X, self.computed_y()),
            Vector2::new((self.computed_x() + CURSOR_OFFSET_X + CURSOR_WIDTH) as f32, self.computed_y() + self.font.char_height)
        )
    }

    pub fn render(&self, graphics: &mut Graphics2D) {
        graphics.draw_rectangle(self.get_rectangle(), Color::BLACK);
    }
}