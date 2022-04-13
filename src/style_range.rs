use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::Graphics2D;
use crate::camera::Camera;
use crate::font::Font;
use crate::line::Line;
use crate::range::Range;
use crate::range_trait::RangeTrait;

#[derive(Clone, Copy)]
pub struct StyleRange {
    pub color: Color,
    pub bold: bool,
    pub underline: bool,
    pub strikethrough: bool, // barré
    pub range: Range,
}

impl Default for StyleRange {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            bold: false,
            underline: false,
            strikethrough: false,
            range: Range::default()
        }
    }
}

impl Debug for StyleRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let start_text = if let Some(start) = self.range.start { start.x.to_string() + "," + &start.y.to_string() } else { "None".to_owned() } ;
        let end_text = if let Some(end) = self.range.end { end.x.to_string() + "," + &end.y.to_string() } else { "None".to_owned() } ;
        write!(f, "Range : {} - {} \nbold: {}\nunderline: {}\nstrikethrough: {}\ncolor: {:?}", start_text, end_text, self.bold, self.underline, self.strikethrough, self.color)
    }
}

impl PartialEq for StyleRange {
    fn eq(&self, other: &Self) -> bool {
        self.range.eq(&other.range)
    }
}

impl StyleRange {
    fn new_with_parameters(start: Vector2<u32>, end: Vector2<u32>, color: Color, bold: bool, underline: bool, strikethrough: bool) -> Self {
        Self {
            range: Range::new(start, end),
            color,
            bold,
            underline,
            strikethrough,
        }
    }

    pub fn new_colored(range: Range, color: Color) -> Self {
        Self {
            range,
            color,
            bold: false,
            underline: false,
            strikethrough: false,
        }
    }

    pub fn new_bold(range: Range) -> Self {
        Self {
            range,
            color: Color::BLACK,
            bold: true,
            underline: false,
            strikethrough: false,
        }
    }

    pub fn new_underline(range: Range) -> Self {
        Self {
            range,
            color: Color::BLACK,
            bold: false,
            underline: true,
            strikethrough: false,
        }
    }

    pub fn new_strikethrough(range: Range) -> Self {
        Self {
            range,
            color: Color::BLACK,
            bold: false,
            underline: false,
            strikethrough: true,
        }
    }
}

impl RangeTrait for StyleRange {
    fn new(start: Vector2<u32>, end: Vector2<u32>,) -> Self {
        Self {
            range: Range::new(start, end),
            color: Color::BLACK,
            bold: false,
            underline: false,
            strikethrough: false,
        }
    }

    fn get_range(&self) -> &Range { &self.range }

    fn start(&mut self, position: Vector2<u32>) {
        self.range.start(position)
    }

    fn end(&mut self, position: Vector2<u32>) {
        self.range.end(position)
    }

    fn reset(&mut self) {
        self.range.reset()
    }

    fn add(&mut self, other: StyleRange) {
        self.range.add(other.range)
    }

    fn include(&self, other: &StyleRange) -> bool {
        self.range.include(&other.range)
    }

    fn is_valid(&self) -> bool {
        self.range.is_valid()
    }

    fn get_id(&self) -> String {
        self.range.get_id()
    }

    fn get_real_start(&self) -> Option<Vector2<u32>> {
        self.range.get_real_start()
    }

    fn get_real_end(&self) -> Option<Vector2<u32>> {
        self.range.get_real_end()
    }

    fn get_ranges_from_drn_line(pattern: &str, lines: &Vec<&str>) -> Vec<Range> {
        Range::get_ranges_from_drn_line(pattern, lines)
    }

    fn get_lines_index(&mut self, lines: &[Line]) -> Vec<(u32, u32)> {
        self.range.get_lines_index(lines)
    }

    fn _render(&mut self, font: Rc<RefCell<Font>>, lines: &[Line], camera: &Camera, graphics: &mut Graphics2D) {
       self.range._render(font, lines, camera, graphics)
    }
}