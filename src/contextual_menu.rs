use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::font::FormattedTextBlock;
use speedy2d::Graphics2D;
use speedy2d::window::{ModifiersState, UserEventSender, VirtualKeyCode};

use crate::camera::Camera;
use crate::cursor::{Cursor, CURSOR_OFFSET_X};
use crate::{EditorEvent, FocusElement, MenuAction, MenuId};
use crate::animation::{Animation, EasingFunction};
use crate::FocusElement::{Editor, Menu};
use crate::font::Font;
use crate::render_helper::draw_rounded_rectangle;

const ITEM_PADDING: f32 = 5.;
const ANIMATION_DURATION: f32 = 100.;

#[derive(Clone)]
pub struct MenuItem {
    pub title: String,
    pub action: MenuAction,
    pub sub_menu: Option<ContextualMenu>
}

impl Debug for MenuItem {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "MenuItem : {}", self.title)
    }
}

impl MenuItem {
    pub fn new(title: &str, action: MenuAction) -> Self { Self { title: title.to_string(), action, sub_menu: Option::None } }
}

#[derive(Clone)]
pub struct ContextualMenu {
    id: MenuId,
    pub is_visible: bool,
    pub items: Vec<MenuItem>,
    pub focus_index: isize,
    system_font: Rc<RefCell<Font>>,
    formated_items: Vec<Rc<FormattedTextBlock>>,
    previous_focus: FocusElement,
    pub event_sender: Option<UserEventSender<EditorEvent>>,
    pub size_animation: Vector2<Option<Animation>>,
    pub focus_y_animation: Option<Animation>,
}

impl ContextualMenu {
    pub fn new(font: Rc<RefCell<Font>>) -> Self {
        Self {
            id: [-1, -1, -1],
            is_visible: false,
            items: vec![],
            focus_index: -1,
            system_font: font,
            formated_items: vec![],
            previous_focus: Editor,
            event_sender: Option::None,
            size_animation: Vector2::new(Option::None, Option::None),
            focus_y_animation: Option::None
        }
    }

    pub fn new_with_items(font: Rc<RefCell<Font>>, items: Vec<MenuItem>) -> Self {
        let mut menu = Self::new(font);
        menu.set_items(items);
        menu
    }

    pub fn open(&mut self) {
        if self.is_visible { return; }
        if self.items.len() == 0 {
            let empty_menu = MenuItem::new("Aucune suggestion", MenuAction::Void);
            self.set_items(vec![empty_menu]);
        }
        self.is_visible = true;
        let start_width = if let Some(animation_width) = &self.size_animation.x { animation_width.value } else { 0. };
        let start_height = if let Some(animation_height) = &self.size_animation.y { animation_height.value } else { 0. };
        let new_animation_width = Animation::new(start_width, self.width(), ANIMATION_DURATION, EasingFunction::SmootherStep, self.event_sender.clone().unwrap());
        let new_animation_height = Animation::new(start_height, self.height(), ANIMATION_DURATION, EasingFunction::SmootherStep, self.event_sender.clone().unwrap());
        self.size_animation.x = Some(new_animation_width);
        self.size_animation.y = Some(new_animation_height);
    }

    pub fn close(&mut self) {
        if !self.is_visible { return; }
        self.is_visible = false;
        let start_width = if let Some(animation_width) = &self.size_animation.x { animation_width.value } else { self.width() };
        let start_height = if let Some(animation_height) = &self.size_animation.y { animation_height.value } else { self.height() };
        let new_animation_width = Animation::new(start_width, 0., ANIMATION_DURATION, EasingFunction::SmootherStep, self.event_sender.clone().unwrap());
        let new_animation_height = Animation::new(start_height, 0., ANIMATION_DURATION, EasingFunction::SmootherStep, self.event_sender.clone().unwrap());
        self.size_animation.x = Some(new_animation_width);
        self.size_animation.y = Some(new_animation_height);
        self.unfocus();
    }

    pub fn handle_key(&mut self, keycode: VirtualKeyCode, modifiers: ModifiersState) {
        match keycode {
            VirtualKeyCode::Up => self.move_up(),
            VirtualKeyCode::Down => self.move_down(),
            VirtualKeyCode::Right => { if self.focus_item_has_submenu() { self.focus_submenu() } else { self.close() } },
            VirtualKeyCode::Left => self.unfocus(),
            VirtualKeyCode::Return => self.select(),
            VirtualKeyCode::Escape => self.close(),
            VirtualKeyCode::Tab => { if !modifiers.shift() { self.move_down() } else { self.move_up() } },
            _ => self.close()
        }
    }

    fn move_up(&mut self) {
        if !self.is_focus() { return; }
        let mut index = self.focus_index as i32 - 1;
        let start_index = if let Some(animation) = &self.focus_y_animation { animation.value } else { self.focus_index as f32 };
        if index < 0 { index += self.items.len() as i32}
        self.set_focus(index as isize);
        self.focus_y_animation = Some(Animation::new(start_index, self.focus_index as f32, ANIMATION_DURATION, EasingFunction::EaseOut, self.event_sender.clone().unwrap()));
    }

    fn move_down(&mut self) {
        if !self.is_focus() { return; }
        let start_index = if let Some(animation) = &self.focus_y_animation { animation.value } else { self.focus_index as f32 };
        self.set_focus((self.focus_index + 1) % self.items.len() as isize);
        self.focus_y_animation = Some(Animation::new(start_index, self.focus_index as f32, ANIMATION_DURATION, EasingFunction::EaseOut, self.event_sender.clone().unwrap()));
    }

    fn set_focus(&mut self, index: isize) {
        self.focus_index = index;
        if let Some(sub_menu) = &mut self.get_focused_item().sub_menu {
            sub_menu.open();
        }
    }

    fn get_focused_item(&mut self) -> &mut MenuItem {
        &mut self.items[self.focus_index as usize]
    }

    fn focus_item_has_submenu(&self) -> bool {
        self.items[self.focus_index as usize].sub_menu.is_some()
    }

    pub fn is_focus(&self) -> bool {
        self.focus_index > -1
    }

    pub fn focus(&mut self) {
        self.focus_index = 0;
        self.event_sender.as_ref().unwrap().send_event(EditorEvent::Focus(Menu(self.id))).unwrap();
    }

    pub fn unfocus(&mut self) {
        self.focus_index = -1;
        self.event_sender.as_ref().unwrap().send_event(EditorEvent::Focus(self.previous_focus)).unwrap();
    }

    pub fn focus_submenu(&mut self) {
        let id = self.id.clone();
        if let Some(sub_menu) = &mut self.items[self.focus_index as usize].sub_menu {
            let mut sub_menu_id = id;
            for level in sub_menu_id.iter_mut() {
                if *level <= -1 {
                    *level = self.focus_index;
                    break }
            }
            sub_menu.previous_focus = Menu(id);
            sub_menu.id = sub_menu_id;
            sub_menu.focus()
        }
    }

    pub fn select(&mut self) {
        if !self.is_focus() { return; }
        let action = self.get_focused_item().action.clone();

        self.event_sender.as_ref().unwrap().send_event(
            EditorEvent::MenuItemSelected(action.clone())
        ).unwrap();
        if action == MenuAction::OpenSubMenu {
            self.focus_submenu();
        } else if action != MenuAction::Void {
            self.close();
        }
    }

    pub fn set_items(&mut self, items: Vec<MenuItem>) {
        self.items = items;
        self.update_content();
    }

    pub fn open_with(&mut self, items: Vec<MenuItem>) {
        self.set_items(items);
        self.open();
        self.focus();
    }

    fn width(&self) -> f32 { self.formated_items.iter().map(|ftb| ftb.width()).max_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap()).unwrap_or(0.) + 2. * 4. * ITEM_PADDING}

    fn height(&self) -> f32 { (self.formated_items.iter().map(|ftb| ftb.height()).max_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap()).unwrap_or(0.) + ITEM_PADDING) * self.items.len() as f32 + ITEM_PADDING}

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
        self.formated_items = self.items
            .iter()
            .map(|item| self.system_font.borrow().layout_text(&item.title))
            .collect();
    }

    pub fn render(&mut self, position: Vector2<f32>, graphics: &mut Graphics2D) {
        if !self.is_visible && self.size_animation.y.is_none() || self.items.len() == 0 { return; }
        let width = self.computed_width();
        let item_height = self.formated_items.iter().map(|ftb| ftb.height()).max_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap()).unwrap_or(0.) + ITEM_PADDING;
        let height = self.computed_height();
        let mut menu_origin = position;
        let editor_size = self.system_font.borrow().editor_size;
        if menu_origin.x + width > editor_size.x { menu_origin.x -= menu_origin.x + width - editor_size.x }
        if menu_origin.y + height > editor_size.y { menu_origin.y -= menu_origin.y + height - editor_size.y }
        let border_color: Color = Color::from_int_rgba(150, 150, 150, 250);
        let highlight_color: Color = Color::from_int_rgba(225, 225, 225, 250);
        const BORDER_WIDTH: f32 = 0.5;
        // draw border
        draw_rounded_rectangle(menu_origin.x - BORDER_WIDTH, menu_origin.y - BORDER_WIDTH, width + 2. * BORDER_WIDTH, height + 2. * BORDER_WIDTH, 8. - BORDER_WIDTH, border_color, graphics);
        // draw background
        draw_rounded_rectangle(menu_origin.x, menu_origin.y, width, height, 8., Color::from_int_rgba(250, 250, 250, 250), graphics);
        for (i, item) in self.items.iter_mut().enumerate() {
            // draw highlight
            if i == self.focus_index as usize {
                let computed_i = if let Some(animated_i) = &self.focus_y_animation { animated_i.value } else { i as f32 };
                draw_rounded_rectangle(menu_origin.x, menu_origin.y + item_height * computed_i, width, item_height + ITEM_PADDING, 10., highlight_color, graphics);
                if let Some(sub_menu) = &mut item.sub_menu {
                    sub_menu.render(Vector2::new(menu_origin.x + width, menu_origin.y + item_height * computed_i), graphics);
                }
            } else if let Some(sub_menu) = &mut item.sub_menu { sub_menu.close() }
            graphics.draw_text(
                menu_origin + Vector2::new(2. * ITEM_PADDING, item_height * (i as f32) + ITEM_PADDING),
                Color::BLACK,
                &self.formated_items[i]
            );
        }
    }
}