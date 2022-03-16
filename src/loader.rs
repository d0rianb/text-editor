use itertools::GroupBy;
use lazy_static::lazy_static;
use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::Graphics2D;
use speedy2d::window::UserEventSender;
use crate::{Animation, EditorEvent};
use crate::animation::EasingFunction;
use crate::render_helper::draw_rounded_line;

const TWO_PI: f32 = std::f32::consts::TAU;
const HALF_PI: f32 = std::f32::consts::PI / 2.;
const ANIMATION_DURATION: f32 = 750.;

// Inifinite circular loader
pub struct Loader {
    rotation_animation: Option<Animation>,
    event_sender: UserEventSender<EditorEvent>,
}

impl Loader {
    pub fn new(es:UserEventSender<EditorEvent> ) -> Self {
        Self {
            event_sender: es.clone(),
            rotation_animation: Some(Animation::new_infinite(-HALF_PI, TWO_PI - HALF_PI, ANIMATION_DURATION, EasingFunction::Bilinear, es.clone()))
        }
    }

    pub fn get_animations(&mut self) -> Vec<&mut Option<Animation>> {
        vec![&mut self.rotation_animation]
    }

    pub fn draw(&self, x: f32, y: f32, radius: f32, bg_color: &Color, graphics: &mut Graphics2D) {
        const LINE_SIZE: f32 = 3.;
        const THETA: f32 = std::f32::consts::FRAC_PI_4;  // opening angle of the loader
        assert!(radius > LINE_SIZE);
        lazy_static! { static ref FG_COLOR: Color = Color::from_int_rgb(90, 160, 220); }
        let angle = self.rotation_animation.as_ref().unwrap().value;
        graphics.draw_circle(Vector2::new(x, y), radius, *FG_COLOR);
        graphics.draw_circle(Vector2::new(x, y), radius - LINE_SIZE, *bg_color);
        graphics.draw_circle_section_triangular_three_color(
            [
                Vector2::new(x, y),
                Vector2::new(x + (radius + LINE_SIZE) * (angle + THETA).cos(), y + (radius + LINE_SIZE) * (angle + THETA).sin()),
                Vector2::new(x + (radius + LINE_SIZE) * angle.cos(), y + (radius + LINE_SIZE) * angle.sin()),
            ],
            [*bg_color; 3],
            [
                Vector2::new(0., -1.),
                Vector2::new(-1.0, 1.0),
                Vector2::new(1.0, 1.0),
            ]
        );
    }
}