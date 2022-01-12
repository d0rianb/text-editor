use std::fmt::{Debug, Formatter};
use std::rc::Rc;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::font::FormattedTextBlock;
use speedy2d::Graphics2D;
use speedy2d::window::UserEventSender;

use crate::camera::Camera;
use crate::cursor::{Cursor, CURSOR_OFFSET_X};
use crate::{EditorEvent, MenuAction};
use crate::animation::{Animation, EasingFunction};
use crate::FocusElement::{Editor, MainMenu};
use crate::font::Font;
use crate::render_helper::draw_rounded_rectangle;

const ITEM_PADDING: f32 = 5.;
const ANIMATION_DURATION: f32 = 100.;

pub struct MenuItem {
    pub title: String,
    pub action: MenuAction,
    pub priority: u32
}

impl Debug for MenuItem {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "MenuItem : {}", self.title)
    }
}

impl MenuItem {
    pub fn new(title: &str, action: MenuAction) -> Self { Self { title: title.to_string(), action, priority: 1 } }
}

pub struct ContextualMenu {
    pub is_visible: bool,
    items: Vec<MenuItem>,
    focus_index: usize,
    system_font: Rc<Font>,
    formated_items: Vec<Rc<FormattedTextBlock>>,
    pub event_sender: Option<UserEventSender<EditorEvent>>,
    pub size_animation: Vector2<Option<Animation>>,
    pub focus_y_animation: Option<Animation>,
}

impl ContextualMenu {
    pub fn new(font: Rc<Font>) -> Self {
        let mut menu = Self {
            is_visible: false,
            items: vec![],
            focus_index: 0,
            system_font: font,
            formated_items: vec![],
            event_sender: Option::None,
            size_animation: Vector2 { x: Option::None, y: Option::None },
            focus_y_animation: Option::None
        };
        menu.update_content();
        menu
    }

    pub fn open(&mut self) {
        if self.is_visible { return; }
        self.is_visible = true;
        self.event_sender.as_ref().unwrap().send_event(EditorEvent::Focus(MainMenu));
        let start_width = if let Some(animation_width) = &self.size_animation.x { animation_width.value } else { 0. };
        let start_height = if let Some(animation_height) = &self.size_animation.y { animation_height.value } else { 0. };
        let new_animation_width = Animation::new(start_width, self.width(), ANIMATION_DURATION, EasingFunction::SmootherStep, self.event_sender.clone());
        let new_animation_height = Animation::new(start_height, self.height(), ANIMATION_DURATION, EasingFunction::SmootherStep, self.event_sender.clone());
        self.size_animation.x = Some(new_animation_width);
        self.size_animation.y = Some(new_animation_height);
    }

    pub fn close(&mut self) {
        if !self.is_visible { return; }
        self.focus_index = 0;
        self.is_visible = false;
        self.event_sender.as_ref().unwrap().send_event(EditorEvent::Focus(Editor));
        let start_width = if let Some(animation_width) = &self.size_animation.x { animation_width.value } else { self.width() };
        let start_height = if let Some(animation_height) = &self.size_animation.y { animation_height.value } else { self.height() };
        let new_animation_width = Animation::new(start_width, 0., ANIMATION_DURATION, EasingFunction::SmootherStep, self.event_sender.clone());
        let new_animation_height = Animation::new(start_height, 0., ANIMATION_DURATION, EasingFunction::SmootherStep, self.event_sender.clone());
        self.size_animation.x = Some(new_animation_width);
        self.size_animation.y = Some(new_animation_height);
    }

    pub fn move_up(&mut self) {
        let mut index = self.focus_index as i32 - 1;
        let start_index = if let Some(animation) = &self.focus_y_animation { animation.value } else { self.focus_index as f32 };
        if index < 0 { index += self.items.len() as i32}
        self.focus_index = index as usize;
        self.focus_y_animation = Some(Animation::new(start_index, self.focus_index as f32, ANIMATION_DURATION, EasingFunction::EaseOut, self.event_sender.clone()));
    }

    pub fn move_down(&mut self) {
        let start_index = if let Some(animation) = &self.focus_y_animation { animation.value } else { self.focus_index as f32 };
        self.focus_index = (self.focus_index + 1) % self.items.len();
        self.focus_y_animation = Some(Animation::new(start_index, self.focus_index as f32, ANIMATION_DURATION, EasingFunction::EaseOut, self.event_sender.clone()));
    }

    pub fn select(&mut self) {
        self.event_sender.as_ref().unwrap().send_event(
            EditorEvent::MenuItemSelected(self.items[self.focus_index].action.clone()));
        self.close();
    }

    pub fn set_items(&mut self, items: Vec<MenuItem>) {
        self.items = items;
        self.update_content();
    }

    pub fn open_with(&mut self, items: Vec<MenuItem>) {
        self.set_items(items);
        self.open();
    }

    fn width(&self) -> f32 { self.formated_items.iter().map(|ftb| ftb.width()).max_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap()).unwrap_or(0.) + 2. * 4. * ITEM_PADDING}

    fn height(&self) -> f32 { (self.formated_items.iter().map(|ftb| ftb.height()).max_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap()).unwrap_or(0.) + ITEM_PADDING) * self.items.len() as f32 + 2. * ITEM_PADDING}

    pub fn computed_width(&self) -> f32 {
        if let Some(animation) = &self.size_animation.x {
            animation.value
        } else {
            self.width()
        }
    }

    pub fn computed_height(&self) -> f32 {
        if let Some(animation) = &self.size_animation.y {
            animation.value
        } else {
            self.height()
        }
    }

    pub fn update_content(&mut self) {
        self.items.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.formated_items = self.items.iter().map(|item| self.system_font.layout_text(&item.title)).to_owned().collect();
    }

    pub fn render(&mut self, cursor: &Cursor, camera: &Camera, graphics: &mut Graphics2D) {
        if !self.is_visible && self.size_animation.y.is_none() || self.items.len() == 0 { return; }
        let width = self.computed_width();
        let item_height = self.formated_items.iter().map(|ftb| ftb.height()).max_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap()).unwrap_or(0.) + ITEM_PADDING;
        let height = self.computed_height();
        let mut menu_origin = Vector2::ZERO - camera.position() + cursor.position() + Vector2::new(CURSOR_OFFSET_X, cursor.font.borrow().char_height);
        let editor_size = self.system_font.editor_size;
        if menu_origin.x + width > editor_size.x { menu_origin.x -= menu_origin.x + width - editor_size.x }
        if menu_origin.y + height > editor_size.y { menu_origin.y -= menu_origin.y + height - editor_size.y }
        let border_color: Color = Color::from_int_rgba(150, 150, 150, 250);
        let highlight_color: Color = Color::from_int_rgba(225, 225, 225, 250);
        const BORDER_WIDTH: f32 = 0.5;
        // draw border
        draw_rounded_rectangle(menu_origin.x - BORDER_WIDTH, menu_origin.y - BORDER_WIDTH, width + 2. * BORDER_WIDTH, height + 2. * BORDER_WIDTH, 8. - BORDER_WIDTH, border_color, graphics);
        // draw background
        draw_rounded_rectangle(menu_origin.x, menu_origin.y, width, height, 8., Color::from_int_rgba(250, 250, 250, 250), graphics);
        for (i, item) in self.items.iter().enumerate() {
            // draw highlight
            if i == self.focus_index {
                let computed_i = if let Some(animated_i) = &self.focus_y_animation { animated_i.value } else { i as f32 };
                draw_rounded_rectangle(menu_origin.x, menu_origin.y + item_height * computed_i, width, item_height + ITEM_PADDING, 10., highlight_color, graphics);
            }
            graphics.draw_text(
                menu_origin + Vector2::new(2. * ITEM_PADDING, item_height * (i as f32) + ITEM_PADDING),
                Color::BLACK,
                &self.formated_items[i]
            );
        }
    }
}