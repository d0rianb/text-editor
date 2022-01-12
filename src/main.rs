#[macro_use]
extern crate derivative;

mod animation;
mod cursor;
mod editor;
mod font;
mod line;
mod range;
mod camera;
mod contextual_menu;
mod render_helper;

use std::thread;
use std::env;
use std::time::{Duration, Instant};

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::window::{KeyScancode, ModifiersState, MouseButton, VirtualKeyCode, WindowCreationOptions, WindowHandler, WindowHelper, WindowPosition, WindowSize, WindowStartupInfo};
use speedy2d::{Graphics2D, Window};
use speedy2d::font::TextAlignment;

use editor::Editor;

const FPS: u64 = 60;
const FRAME_DURATION: u64 = 1000 / FPS; // ms

#[derive(PartialEq, Debug, Clone)]
pub enum MenuAction {
    Save(String),
    SaveTo(String),
    Open(String),
    Exit,
    CancelChip,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum FocusElement { Editor, MainMenu }

#[derive(PartialEq, Debug, Clone)]
pub enum EditorEvent { Udpate, Redraw, Focus(FocusElement), MenuItemSelected(MenuAction)}


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
        self.editor.set_animation_event_sender(Some(event_sender.clone()));
        helper.request_redraw();
        thread::spawn(move || {
            loop {
                event_sender.send_event(EditorEvent::Udpate).unwrap();
                thread::sleep(Duration::from_millis(FRAME_DURATION));
            }
        });
    }

    fn on_user_event(&mut self, helper: &mut WindowHelper<EditorEvent>, user_event: EditorEvent) {
        match user_event {
            EditorEvent::Redraw => helper.request_redraw(),
            EditorEvent::Udpate => {
                self.editor.update(self.tick_timestamp.elapsed().as_millis() as f32);
                self.tick_timestamp = Instant::now();
            },
            EditorEvent::Focus(focus_element) => self.focus = focus_element,
            EditorEvent::MenuItemSelected(item) => match item {
                MenuAction::Save(path) => println!("Save item selected to {}", path),
                MenuAction::SaveTo(path) => println!("Save to {}", path),
                MenuAction::Open(path) => println!("Open {}", path),
                MenuAction::Exit => helper.terminate_loop(),
                MenuAction::CancelChip => {},
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
        if self.focus != FocusElement::Editor { return; }
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
        if self.focus == FocusElement::MainMenu {
            let menu = &mut self.editor.menu;
            match virtual_key_code {
                Some(VirtualKeyCode::Up) => menu.move_up(),
                Some(VirtualKeyCode::Down) => menu.move_down(),
                Some(VirtualKeyCode::Return) => menu.select(),
                Some(VirtualKeyCode::Escape) => menu.close(),
                Some(VirtualKeyCode::Tab) => { if !self.editor.modifiers.shift() { menu.move_down() } else { menu.move_up() } },
                _ => self.editor.menu.close()
            }
        } else {
            if self.editor.modifiers.logo() && self.editor.modifiers.alt() {
                // Handle ctrl-alt shortcuts
                match virtual_key_code {
                    Some(VirtualKeyCode::Right) => self.editor.set_line_alignement(TextAlignment::Right),
                    Some(VirtualKeyCode::Left) => self.editor.set_line_alignement(TextAlignment::Left),
                    Some(VirtualKeyCode::Up) => self.editor.set_line_alignement(TextAlignment::Center),
                    _ => {}
                }
            } else {
                match virtual_key_code {
                    Some(VirtualKeyCode::Right) => self.editor.move_cursor_relative(1, 0),
                    Some(VirtualKeyCode::Left) => self.editor.move_cursor_relative(-1, 0),
                    Some(VirtualKeyCode::Up) => self.editor.move_cursor_relative(0, -1),
                    Some(VirtualKeyCode::Down) => self.editor.move_cursor_relative(0, 1),
                    Some(VirtualKeyCode::Backspace) => self.editor.delete_char(),
                    Some(VirtualKeyCode::Delete) => {
                        self.editor.move_cursor_relative(1, 0);
                        self.editor.delete_char();
                    },
                    Some(VirtualKeyCode::Return) => self.editor.new_line(),
                    Some(VirtualKeyCode::Escape) => self.editor.menu.close(),
                    Some(VirtualKeyCode::Tab) => { if self.editor.modifiers.alt() { self.editor.menu.open() } },
                    _ => { return; },
                }
            }
        }
        self.editor.update_text_layout();
        helper.request_redraw();
    }

    fn on_keyboard_char(&mut self, helper: &mut WindowHelper<EditorEvent>, unicode_codepoint: char) {
        if unicode_codepoint >= ' '  && unicode_codepoint <= '~' {
            self.editor.add_char(unicode_codepoint.to_string());
            self.editor.update_text_layout();
            helper.request_redraw();
        }
    }

    fn on_keyboard_modifiers_changed(&mut self, _helper: &mut WindowHelper<EditorEvent>, state: ModifiersState) {
        self.editor.modifiers = state;
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
    let mut editor = Editor::new(1200., 800.);
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
