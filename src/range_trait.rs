use std::cell::RefCell;
use std::rc::Rc;
use speedy2d::dimen::Vector2;
use speedy2d::Graphics2D;
use crate::camera::Camera;
use crate::font::Font;
use crate::line::Line;
use crate::range::Range;

pub trait RangeTrait {
    fn new(start: Vector2<u32>, end: Vector2<u32>) -> Self;

    fn get_range(&self) -> &Range;

    fn start(&mut self, position: Vector2<u32>);

    fn end(&mut self, position: Vector2<u32>);

    fn reset(&mut self);

    fn add(&mut self, other: Self);

    fn include(&self, other: &Self) -> bool;

    fn is_valid(&self) -> bool;

    fn get_id(&self) -> String;

    fn get_real_start(&self) -> Option<Vector2<u32>>;

    fn get_real_end(&self) -> Option<Vector2<u32>>;

    fn get_ranges_from_drn_line(pattern: &str, lines: &Vec<&str>) -> Vec<Range>;

    fn get_lines_index(&mut self, lines: &[Line]) -> Vec<(u32, u32)>;

    fn _render(&mut self, font: Rc<RefCell<Font>>, lines: &[Line], camera: &Camera, graphics: &mut Graphics2D);
}