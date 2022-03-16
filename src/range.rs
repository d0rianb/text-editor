use std::cell::RefCell;
use std::cmp;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use std::slice::Split;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::Graphics2D;
use speedy2d::shape::Rectangle;

use crate::camera::Camera;
use crate::font::Font;
use crate::line::Line;

pub fn get_line_length(i: u32, lines: &[Line]) -> u32 {
    if i + 1 > lines.len() as u32 { return 0; } // Prevent overflow
    lines[i as usize].buffer.len() as u32
}

#[derive(Clone, Copy)]
pub struct Range {
    pub start: Option<Vector2<u32>>,
    pub end: Option<Vector2<u32>>,
}

impl Default for Range {
    fn default() -> Self {
        Self {
            start: Option::None,
            end: Option::None,
        }
    }
}

impl Debug for Range {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let start_text = if let Some(start) = self.start { start.x.to_string() + "," + &start.y.to_string() } else { "None".to_owned() } ;
        let end_text = if let Some(end) = self.end { end.x.to_string() + "," + &end.y.to_string() } else { "None".to_owned() } ;
        write!(f, "Range : {} - {}", start_text, end_text)
    }
}

impl PartialEq for Range {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}

pub fn vector_max(v1: Vector2<u32>, v2: Vector2<u32>) -> Vector2<u32> {
    if v1.y < v2.y { return v2 }
    if v1.y > v2.y { return v1  }
    if v1.x < v2.x { return v2 }
    else { return v1 }
}

pub fn vector_min(v1: Vector2<u32>, v2: Vector2<u32>) -> Vector2<u32> {
    if vector_max(v1, v2) == v2 { v1 } else { v2 }
}

impl Range {
    pub fn new(start: Vector2<u32>, end: Vector2<u32>) -> Self {
        Self {
            start: Some(start),
            end: Some(end),
        }
    }

    pub fn start(&mut self, position: Vector2<u32>) {
        self.start = Some(Vector2::new(position.x, position.y));
    }

    pub fn end(&mut self, position: Vector2<u32>) {
        self.end = Some(Vector2::new(position.x, position.y));
    }

    pub fn reset(&mut self) {
        self.start = Option::None;
        self.end = Option::None;
    }

    pub fn add(&mut self, other: Range) {
        if !other.is_valid() { return; }
        if !self.is_valid() {
            self.start(other.start.unwrap());
            self.end(other.end.unwrap());
            return;
        }
        let start= vector_min(self.start.unwrap(), other.start.unwrap());
        let end= vector_max(self.end.unwrap(), other.end.unwrap());
        self.start(start);
        self.end(end);
    }

    pub fn include(&self, other: &Range) -> bool {
        if !self.is_valid() || !other.is_valid() { return false; }
        vector_min(self.start.unwrap(), other.start.unwrap()) == self.start.unwrap()
            && vector_max(self.end.unwrap(), other.end.unwrap()) == self.end.unwrap()
    }

    pub fn is_valid(&self) -> bool {
        self.start.is_some() && self.end.is_some() && self.start != self.end
    }

    pub fn get_id(&self) -> String {
        if !self.is_valid() { return "Invalid range".to_string() }
        let start = self.start.unwrap();
        let end = self.end.unwrap();
        format!("{}-{}-{}-{}", start.x, start.y, end.x, end.y)
    }

    pub fn get_real_start(&self) -> Option<Vector2<u32>> {
        if !self.is_valid() { return Option::None; }
        Some(vector_min(self.start.unwrap(), self.end.unwrap()))
    }

    pub fn get_real_end(&self) -> Option<Vector2<u32>> {
        if !self.is_valid() { return Option::None; }
        Some(vector_max(self.start.unwrap(), self.end.unwrap()))
    }

    pub fn get_ranges_from_drn_line(pattern: &str, lines: &Vec<&str>) -> Vec<Range> {
        let mut result = vec![];
        let mut line = lines
            .iter()
            .find(|line| (**line).starts_with(pattern))
            .unwrap_or(&" ")
            .to_string();
        line = line.replace(pattern, "");
        for range_str in line.split(",") {
            if range_str == line { break; }
            let numbers: Vec<&str> = range_str.trim().split("-").collect();
            let mut parsed_number = numbers
                .iter()
                .map(|n| n.parse::<u32>());
            if parsed_number.clone().any(|n| n.is_err()) { continue }
            let coords: Vec<u32> = parsed_number.map(|n| n.unwrap()).collect();
            result.push(Range::new(Vector2::new(coords[0], coords[1]),  Vector2::new(coords[2], coords[3])))
        }
        result
    }

    pub fn get_lines_index(&mut self, lines: &[Line]) -> Vec<(u32, u32)> {
        // relative index of selection starting in the self.start.y index
        if !self.is_valid() { return vec![]; }
        let start = self.get_real_start().unwrap();
        let end = self.get_real_end().unwrap();
        let mut result = vec![];
        for y in start.y..=end.y {
            if y == start.y {
                if start.y == end.y {
                    result.push((start.x, end.x)) } else { result.push((start.x, get_line_length(y, lines)))
                }
            } else if y == end.y { result.push((0, end.x)) } else { result.push((0, get_line_length(y, lines))) }
        }
        result
    }

    pub fn _render(&mut self, font: Rc<RefCell<Font>>, lines: &[Line], camera: &Camera, graphics: &mut Graphics2D) {
        if !self.is_valid() { return; }
        let font_width = font.borrow().char_width;
        let font_height = font.borrow().char_height;
        let initial_y = self.get_real_start().unwrap().y as f32 * font_height - camera.computed_y();
        for (i, indices) in self.get_lines_index(lines).iter().enumerate() { // TODO: cache ?
            let line_y = initial_y + i as f32 * font_height;
            let line = &lines[self.get_real_start().unwrap().y as usize + i];
            let line_offset = line.alignment_offset;
            let line_camera = Camera::from_with_offset(camera, Vector2::new(-line_offset, 0.));
            graphics.draw_rectangle(
                Rectangle::new(
                    Vector2::new(indices.0 as f32 * font_width - line_camera.computed_x(), line_y),
                    Vector2::new(indices.1 as f32 * font_width - line_camera.computed_x(), line_y + font_height),
                ),
                Color::from_int_rgba(100, 100, 100, 50),
            )
        }
    }

}