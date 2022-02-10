use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use lazy_static::lazy_static;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::font::{FormattedTextBlock, TextOptions};
use speedy2d::Graphics2D;
use speedy2d::shape::Rectangle;
use speedy2d::window::{ModifiersState, UserEventSender, VirtualKeyCode};

use crate::{Editable, EditorEvent, FocusElement, MenuId};
use crate::menu_actions::MenuAction;
use crate::animation::{Animation, EasingFunction};
use crate::font::Font;
use crate::input::{Input, Validator};
use crate::render_helper::{draw_rectangle, draw_rounded_rectangle, draw_rounded_rectangle_with_border};

const ITEM_PADDING: f32 = 5.;
const SEPARATOR_HEIGHT_RATIO: f32 = 1. / 10.;
const ANIMATION_DURATION: f32 = 100.;

pub struct MenuItem {
    pub title: String,
    pub action: MenuAction,
    pub sub_menu: Option<ContextualMenu>,
    pub input: Option<Input>
}

impl Debug for MenuItem {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "MenuItem : {} | {}", self.title, self.action)
    }
}

impl PartialEq for MenuItem {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title && self.action == other.action
    }
}

impl MenuItem {
    pub fn new(title: &str, action: MenuAction) -> Self {
        Self {
            title: title.to_string(),
            action,
            sub_menu: Option::None,
            input: Option::None,
        }
    }

    pub fn new_with_submenu(title: &str, sub_menu: ContextualMenu) -> Self {
        Self {
            title: title.to_string(),
            action: MenuAction::OpenSubMenu,
            sub_menu: Some(sub_menu),
            input: Option::None,
        }
    }

    pub fn separator() -> Self {
        Self {
            title: "#separator#".to_string(),
            action: MenuAction::Separator,
            sub_menu: Option::None,
            input: Option::None,
        }
    }

}

pub struct ContextualMenu {
    pub id: MenuId,
    pub is_visible: bool,
    pub items: Vec<MenuItem>,
    pub focus_index: isize,
    system_font: Rc<RefCell<Font>>,
    formatted_items: Vec<Rc<FormattedTextBlock>>,
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
            formatted_items: vec![],
            previous_focus: FocusElement::Editor,
            event_sender: Option::None,
            size_animation: Vector2::new(Option::None, Option::None),
            focus_y_animation: Option::None,
        }
    }

    pub fn new_with_items(font: Rc<RefCell<Font>>, es: UserEventSender<EditorEvent>, items: Vec<MenuItem>) -> Self {
        let mut menu = Self::new(font);
        menu.event_sender = Some(es);
        menu.set_items(items);
        menu
    }

    pub fn open(&mut self) {
        if self.is_visible { return; }
        if self.items.is_empty() {
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
       self.internal_close();
        self.unfocus();
    }

    // Close the submenu without unfocus the current one
    pub fn close_submenu(&mut self) {
        self.internal_close();
    }

    fn internal_close(&mut self) {
        if !self.is_visible { return; }
        self.is_visible = false;
        let start_width = if let Some(animation_width) = &self.size_animation.x { animation_width.value } else { self.width() };
        let start_height = if let Some(animation_height) = &self.size_animation.y { animation_height.value } else { self.height() };
        let new_animation_width = Animation::new(start_width, 0., ANIMATION_DURATION, EasingFunction::SmootherStep, self.event_sender.clone().unwrap());
        let new_animation_height = Animation::new(start_height, 0., ANIMATION_DURATION, EasingFunction::SmootherStep, self.event_sender.clone().unwrap());
        self.size_animation.x = Some(new_animation_width);
        self.size_animation.y = Some(new_animation_height);
    }

    pub fn handle_key(&mut self, keycode: VirtualKeyCode, modifiers: ModifiersState) {
        if self.get_focused_item().action == MenuAction::Information { // Handle informative toggle
            if keycode == VirtualKeyCode::Escape || (modifiers.logo() && keycode == VirtualKeyCode::I) { self.close() }
            else { return; }
        }
        match keycode {
            VirtualKeyCode::Up => self.move_up(),
            VirtualKeyCode::Down => self.move_down(),
            VirtualKeyCode::Right => { if self.focus_item_has_submenu() { self.focus_submenu() } else { self.close() } },
            VirtualKeyCode::Left => self.unfocus(),
            VirtualKeyCode::Return => self.select(),
            VirtualKeyCode::Escape => self.event_sender.as_ref().unwrap().send_event(EditorEvent::MenuItemSelected(MenuAction::CloseMenu)).unwrap(),
            VirtualKeyCode::Tab => { if !modifiers.shift() { self.move_down() } else { self.move_up() } },
            VirtualKeyCode::LShift
            | VirtualKeyCode::RShift
            | VirtualKeyCode::LControl
            | VirtualKeyCode::RControl
            | VirtualKeyCode::LWin
            | VirtualKeyCode::RWin
            | VirtualKeyCode::LAlt
            | VirtualKeyCode::RAlt => {}, // Prevent closing the menu while pressing a modifier
            _ => self.close(),
        }
    }

    pub fn send_key_to_input(&mut self, keycode: VirtualKeyCode, modifiers: ModifiersState) {
        if let Some(input) =  &mut self.get_focused_item().input {
            input.editor.modifiers = modifiers;
            match keycode {
                VirtualKeyCode::Up => self.move_up(),
                VirtualKeyCode::Down => self.move_down(),
                _ => input.handle_key(keycode)
            }
        } else {
            self.handle_key(keycode, modifiers);
        }
    }

    fn move_up(&mut self) {
        if !self.is_focus() { return; }
        let mut index = self.focus_index as i32 - 1;
        let start_y = if let Some(animation) = &self.focus_y_animation { animation.value } else { self.get_item_offset_y(self.focus_index as usize) };
        self.set_focus(index as isize);
        self.focus_y_animation = Some(Animation::new(start_y, self.get_item_offset_y(self.focus_index as usize), ANIMATION_DURATION, EasingFunction::EaseOut, self.event_sender.clone().unwrap()));
    }

    fn move_down(&mut self) {
        if !self.is_focus() { return; }
        let start_y = if let Some(animation) = &self.focus_y_animation { animation.value } else { self.get_item_offset_y(self.focus_index as usize) };
        self.set_focus(self.focus_index + 1);
        self.focus_y_animation = Some(Animation::new(start_y, self.get_item_offset_y(self.focus_index as usize), ANIMATION_DURATION, EasingFunction::EaseOut, self.event_sender.clone().unwrap()));
    }

    fn set_focus(&mut self, index: isize) {
        let dir = index - self.focus_index;
        self.focus_index = index % self.items.len() as isize;
        if self.focus_index < 0 { self.focus_index += self.items.len() as isize }
        for item in self.items.iter_mut() {
            if let Some(sub_menu) = &mut item.sub_menu { sub_menu.close_submenu() }
        }
        let focus_item = self.get_focused_item();
        if focus_item.action == MenuAction::Separator { return self.set_focus(index + dir) }
        if let Some(sub_menu) = &mut focus_item.sub_menu {
            sub_menu.open()
        }

        if let Some(input) = &mut focus_item.input {
            match &focus_item.action {
                MenuAction::SaveWithInput(path)
                | MenuAction::NewFileWithInput(path)
                | MenuAction::OpenWithInput(path) => { input.set_placeholder(path); input.set_validator(Validator::File) },
                _ => {}
            }
            input.focus();
        }
    }

    pub fn get_animations(&mut self) -> Vec<&mut Option<Animation>> {
        let mut animations = vec![&mut self.size_animation.x, &mut self.size_animation.y, &mut self.focus_y_animation];
        for items in self.items.iter_mut() {
            if let Some(input) = &mut items.input {
                for animation in input.get_animations() {
                    animations.push(animation)
                }
            }
            if let Some(sub_menu) = &mut items.sub_menu {
               for animation in sub_menu.get_animations() {
                   animations.push(animation);
               }
            }
        }
        animations
    }

    pub fn get_focused_item(&mut self) -> &mut MenuItem {
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
        self.event_sender.as_ref().unwrap().send_event(EditorEvent::Focus(FocusElement::Menu(self.id))).unwrap();
        self.set_focus(0);
    }

    pub fn unfocus(&mut self) {
        self.focus_index = -1;
        self.event_sender.as_ref().unwrap().send_event(EditorEvent::Focus(self.previous_focus)).unwrap();
        if self.previous_focus == FocusElement::Editor { self.internal_close(); }
    }

    pub fn focus_submenu(&mut self) {
        if let Some(sub_menu) = &mut self.items[self.focus_index as usize].sub_menu {
            sub_menu.focus();
        } else if let Some(_input) = &mut self.items[self.focus_index as usize].input {
           self.event_sender.as_ref().unwrap().send_event(EditorEvent::Focus(FocusElement::MenuInput(self.id))).unwrap()
        }
    }

    pub fn select(&mut self) {
        if !self.is_focus() { return; }
        let action = self.get_focused_item().action.clone();
        self.event_sender.as_ref().unwrap().send_event(EditorEvent::MenuItemSelected(action.clone())).unwrap();
        match action {
            MenuAction::OpenSubMenu => self.focus_submenu(),
            MenuAction::Void => {},
            _ => self.close()
        }
    }

    fn define_id(&mut self) {
        let id = self.id;
        for (i, item) in self.items.iter_mut().enumerate() {
            if let Some(sub_menu) = &mut item.sub_menu {
                let mut sub_menu_id = id;
                for level in sub_menu_id.iter_mut() {
                    if *level <= -1 {
                        *level = i as isize;
                        break;
                    }
                }
                sub_menu.previous_focus = FocusElement::Menu(id);
                sub_menu.id = sub_menu_id;
                sub_menu.define_id();
            }
        }
    }

    pub fn define_input(&mut self) {
        let es = self.event_sender.clone().unwrap();
        let mut id = self.id;
        for item in &mut self.items {
            if let Some(sub_menu) = &mut item.sub_menu {
                sub_menu.define_input();
                id = sub_menu.id;
            }
            let action_name = item.action.to_string();
            if action_name.contains("WithInput") {
                if item.input.is_none() {
                    let action = MenuAction::get_fn(&item.action);
                    item.input = Some(Input::new(id, action, es.clone()));
                    if item.action == MenuAction::FindAndJumpWithInput { item.input.as_mut().unwrap().set_intermediate_result() }
                } else {
                    item.input.as_mut().unwrap().menu_id = id;
                }
            }
        }
    }

    pub fn set_items(&mut self, items: Vec<MenuItem>) {
        self.items = items;
        self.define_id();
        self.define_input();
        self.update_content();
    }

    pub fn open_with(&mut self, items: Vec<MenuItem>) {
        self.set_items(items);
        self.open();
        self.focus();
    }

    fn width(&self) -> f32 { self.formatted_items.iter().map(|ftb| ftb.width()).max_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap()).unwrap_or(0.) + 8. * ITEM_PADDING }

    fn height(&self) -> f32 {
        let separator_count = self.items.iter().filter(|&item| item.action == MenuAction::Separator).count();
        let item_height = self.formatted_items.iter().map(|ftb| ftb.height()).max_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap()).unwrap_or(0.) + ITEM_PADDING;
        item_height * (self.items.len() - separator_count) as f32 + ITEM_PADDING + separator_count as f32 * SEPARATOR_HEIGHT_RATIO * item_height
    }

    pub fn computed_width(&self) -> f32 {
        if let Some(animation) = &self.size_animation.x { animation.value } else { self.width() }
    }

    pub fn computed_height(&self) -> f32 {
        if let Some(animation) = &self.size_animation.y { animation.value } else { self.height() }
    }

    fn get_item_offset_y<T: Into<usize>>(&self, i: T) -> f32 {
        let index = i.into();
        let separator_count = self.items[..index].iter().filter(|&item| item.action == MenuAction::Separator).count();
        let item_height = self.formatted_items.iter().map(|ftb| ftb.height()).max_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap()).unwrap_or(0.) + ITEM_PADDING;
        item_height * (index - separator_count) as f32  + separator_count as f32 * item_height * SEPARATOR_HEIGHT_RATIO
    }

    pub fn update_content(&mut self) {
        self.formatted_items = self.items
            .iter()
            .map(|item| self.system_font.borrow().layout_text(&item.title, TextOptions::default())) // TODO: wrap on max size
            .collect();
    }

    pub fn render(&self, position: Vector2<f32>, graphics: &mut Graphics2D) {
        if !self.is_visible && self.size_animation.y.is_none() || self.items.is_empty() { return; }
        let width = self.computed_width();
        let item_height = self.formatted_items.iter().map(|ftb| ftb.height()).max_by(|x, y| x.abs().partial_cmp(&y.abs()).unwrap()).unwrap_or(0.) + ITEM_PADDING;
        let height = self.computed_height();
        let mut menu_origin = position;
        let editor_size = self.system_font.borrow().editor_size;
        if menu_origin.x + width > editor_size.x { menu_origin.x -= menu_origin.x + width - editor_size.x }
        if menu_origin.y + height > editor_size.y { menu_origin.y -= menu_origin.y + height - editor_size.y }
        let highlight_color: Color = Color::from_int_rgba(225, 225, 225, 255);
        const BORDER_WIDTH: f32 = 0.5;
        lazy_static! {
            static ref BG_COLOR: Color = Color::from_int_rgb(250, 250, 250);
            static ref BORDER_COLOR: Color = Color::from_int_rgb(150, 150, 150);
            static ref SEPARATOR_COLOR: Color = Color::from_int_rgb(200, 200, 200);
        }
        // draw background
        draw_rounded_rectangle_with_border(menu_origin.x, menu_origin.y, width, height, 8., BORDER_WIDTH, *BG_COLOR, *BORDER_COLOR, graphics);
        for (i, item) in self.items.iter().enumerate() {
            let y = menu_origin.y + self.get_item_offset_y(i);
            if item.action == MenuAction::Separator {
                // Handle separators
                const SEPARATOR_HEIGHT: f32 = 1.;
                draw_rectangle(menu_origin.x + ITEM_PADDING * 3., y + item_height * SEPARATOR_HEIGHT_RATIO / 2. - SEPARATOR_HEIGHT / 2. + ITEM_PADDING / 2., width - ITEM_PADDING * 6., SEPARATOR_HEIGHT, *SEPARATOR_COLOR, graphics);
                continue;
            }
            // draw highlight
            if i == self.focus_index as usize && item.action != MenuAction::Information {
                let offset_y = if let Some(animated_i) = &self.focus_y_animation { animated_i.value } else { self.get_item_offset_y(i) };
                let y = menu_origin.y + offset_y;
                draw_rounded_rectangle(menu_origin.x, y, width, item_height + ITEM_PADDING, 10., highlight_color, graphics);
                if let Some(sub_menu) = &item.sub_menu {
                    sub_menu.render(Vector2::new(menu_origin.x + width, y), graphics);
                } else if let Some(input) = &item.input {
                    input.render(menu_origin.x + width, y, graphics);
                }
            }
        }
        // Draw text in order to not overlap
        for (i, item) in self.items.iter().enumerate() {
            graphics.set_clip(Some(
                Rectangle::new(
                    Vector2::new(menu_origin.x as i32, menu_origin.y as i32),
                    Vector2::new((menu_origin.x + width) as i32, (menu_origin.y + height) as i32)
                )
            ));
            if item.action == MenuAction::Separator { continue; }
            graphics.draw_text(
                menu_origin + Vector2::new(2. * ITEM_PADDING, self.get_item_offset_y(i) + ITEM_PADDING),
                Color::BLACK,
                &self.formatted_items[i]
            );
            graphics.set_clip(Option::None);
        }
    }
}