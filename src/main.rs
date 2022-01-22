#[macro_use]
extern crate derivative;
use strum_macros::EnumString;

mod editor;
mod cursor;
mod font;
mod line;
mod animation;
mod range;
mod camera;
mod contextual_menu;
mod render_helper;
mod input;
mod editable;

use std::{fmt, thread};
use std::env;
use std::time::{Duration, Instant};

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::window::{KeyScancode, ModifiersState, MouseButton, VirtualKeyCode, WindowCreationOptions, WindowHandler, WindowHelper, WindowPosition, WindowSize, WindowStartupInfo};
use speedy2d::{Graphics2D, Window};

use editor::Editor;
use crate::animation::Animation;
use crate::editable::Editable;
use crate::editor::{EDITOR_OFFSET_TOP, EDITOR_PADDING};

const FPS: u64 = 60;
const FRAME_DURATION: u64 = 1000 / FPS; // ms


#[derive(PartialEq, Debug, Clone)]
pub enum MenuAction {
    Open(String),
    OpenWithInput,
    Save(String),
    SaveWithInput,
    Void,
    Exit,
    CancelChip,
    Underline,
    Bold,
    OpenSubMenu,
    CloseMenu,
    PrintWithInput,
    Print(String),
    NewFile(String),
    NewFileWithInput(String),
}

impl fmt::Display for MenuAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{:?}", self) }
}

impl MenuAction {
    pub fn get_fn(action: &MenuAction) -> MenuActionFn {
        match action {
            MenuAction::OpenWithInput => MenuAction::Open,
            MenuAction::SaveWithInput => MenuAction::Save,
            MenuAction::PrintWithInput => MenuAction::Print,
            MenuAction::NewFileWithInput(_) => MenuAction::NewFile,
            _ => MenuAction::Print
        }
    }
}

type MenuActionFn = fn(String) -> MenuAction;
type MenuId = [isize; 3]; // Support 3 nested menu

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum FocusElement { Editor, Menu(MenuId), MenuInput(MenuId) }

#[derive(PartialEq, Debug, Clone)]
pub enum EditorEvent {
    Update, Redraw, Focus(FocusElement), MenuItemSelected(MenuAction)
}

struct EditorWindowHandler {
    editor: Editor,
    last_editor_size: Vector2<u32>,
    tick_timestamp: Instant,
    mouse_button_pressed: (bool, bool), // (Left, Right)
    mouse_position: Vector2<f32>,
    focus: FocusElement,
}

impl WindowHandler<EditorEvent> for EditorWindowHandler {
    fn on_start(&mut self, helper: &mut WindowHelper<EditorEvent>, _info: WindowStartupInfo) {
        let event_sender = helper.create_user_event_sender();
        self.editor.event_sender = Some(event_sender.clone());
        self.editor.set_event_sender(Some(event_sender.clone()));
        helper.request_redraw();
        thread::spawn(move || {
            loop {
                event_sender.send_event(EditorEvent::Update).unwrap();
                thread::sleep(Duration::from_millis(FRAME_DURATION));
            }
        });
    }

    fn on_user_event(&mut self, helper: &mut WindowHelper<EditorEvent>, user_event: EditorEvent) {
        match user_event {
            EditorEvent::Redraw => helper.request_redraw(),
            EditorEvent::Update => {
                self.editor.update(self.tick_timestamp.elapsed().as_millis() as f32);
                self.tick_timestamp = Instant::now();
            },
            EditorEvent::Focus(focus_element) => self.focus = focus_element,
            EditorEvent::MenuItemSelected(item) => match item {
                MenuAction::Void => {},
                MenuAction::Exit => helper.terminate_loop(),
                MenuAction::CancelChip => self.editor.cancel_chip(),
                MenuAction::Open(path) => self.editor.load_file(&path),
                MenuAction::Save(path) => self.editor.save_to_file(&path),
                MenuAction::Underline => self.editor.underline(),
                MenuAction::Bold => self.editor.bold(),
                MenuAction::OpenSubMenu => {},
                MenuAction::CloseMenu => self.editor.menu.close(),
                MenuAction::Print(text) => println!("{}", text),
                _ => {}
            }
        }
    }

    fn on_resize(&mut self, _helper: &mut WindowHelper<EditorEvent>, size_pixels: Vector2<u32>) {
        if self.last_editor_size != size_pixels {
            self.editor.on_resize(size_pixels);
            self.editor.update_text_layout();
        }
        self.last_editor_size = size_pixels;
    }

    fn on_draw(&mut self, _helper: &mut WindowHelper<EditorEvent>, graphics: &mut Graphics2D) {
        graphics.clear_screen(Color::WHITE);
        self.editor.render(graphics);
    }

    fn on_mouse_move(&mut self, helper: &mut WindowHelper<EditorEvent>, position: Vector2<f32>) {
        self.mouse_position = position.clone();
        if self.mouse_button_pressed.0 || self.editor.modifiers.shift() {
            self.editor.update_selection(position);
            helper.request_redraw();
        }
    }

    fn on_mouse_button_down(&mut self, helper: &mut WindowHelper<EditorEvent>, button: MouseButton) {
        match button {
            MouseButton::Left => {
                self.mouse_button_pressed.0 = true;
                self.editor.selection.reset();
                let index_position = self.editor.get_mouse_position_index(self.mouse_position);
                self.editor.move_cursor(Vector2::new(index_position.x, index_position.y));
                self.editor.begin_selection();
            },
            MouseButton::Right => self.mouse_button_pressed.1 = true,
            _ => ()
        }
        helper.request_redraw();
    }

    fn on_mouse_button_up(&mut self, _helper: &mut WindowHelper<EditorEvent>, button: MouseButton) {
        match button {
            MouseButton::Left => self.mouse_button_pressed.0 = false,
            MouseButton::Right => self.mouse_button_pressed.1 = false,
            _ => ()
        }
    }

    fn on_key_down(&mut self, helper: &mut WindowHelper<EditorEvent>, virtual_key_code: Option<VirtualKeyCode>, _scancode: KeyScancode) {
        let modifiers = self.editor.modifiers.clone();
        // TODO: Move in struct impl
        if let Some(keycode) = virtual_key_code {
            match self.focus {
                FocusElement::Menu(id) => self.editor.get_menu(id).handle_key(keycode, modifiers),
                FocusElement::Editor => self.editor.handle_key(keycode),
                FocusElement::MenuInput(id) => {
                    self.editor.get_menu(id).send_key_to_input(keycode, modifiers)
                },
            }
        }
        helper.request_redraw();
    }

    fn on_keyboard_char(&mut self, helper: &mut WindowHelper<EditorEvent>, unicode_codepoint: char) {
        if unicode_codepoint >= ' '  && unicode_codepoint <= '~' || unicode_codepoint >= '¡' {
            match self.focus {
                FocusElement::Editor => {
                        self.editor.add_char(unicode_codepoint.to_string());
                        self.editor.update_text_layout();
                    }

                FocusElement::MenuInput(id) => {
                    let input = self.editor.get_menu(id).get_focused_item().input.as_mut().unwrap();
                    input.add_char(unicode_codepoint.to_string());
                    input.update_text_layout();
                }
                _ => {}
            }
            helper.request_redraw();

        }
    }

    fn on_keyboard_modifiers_changed(&mut self, _helper: &mut WindowHelper<EditorEvent>, state: ModifiersState) {
        self.editor.modifiers = state.clone();
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // For transparenting the titlebar : set
    //      ns_window.setTitlebarAppearsTransparent_(YES);
    //      masks |= NSWindowStyleMask::NSFullSizeContentViewWindowMask;
    let window = Window::new_with_user_events(
        "Text Editor",
        WindowCreationOptions::new_windowed(
            WindowSize::ScaledPixels((600., 400.).into()),
            Some(WindowPosition::Center)
        )
    ).unwrap();
    let mut editor = Editor::new(1200., 800., Vector2::new(0., EDITOR_OFFSET_TOP), EDITOR_PADDING); // on mac dpr is 2 so the real size is 1200, 800
    if args.len() > 1 {
        let filename = &args[1];
        editor.load_file(filename);
    }

    let window_handler = EditorWindowHandler {
        editor,
        last_editor_size: (1200, 800).into(),
        tick_timestamp: Instant::now(),
        mouse_button_pressed: (false, false),
        mouse_position: Vector2::new(0., 0.),
        focus: FocusElement::Editor
    };

    window.run_loop(window_handler);
}
