#[macro_use]
extern crate derivative;
extern crate core;

mod editor;
mod cursor;
mod font;
mod line;
mod animation;
mod range;
mod selection;
mod camera;
mod contextual_menu;
mod render_helper;
mod input;
mod editable;
mod menu_actions;
mod stats;
mod open_ai_wrapper;
mod loader;
mod tesl;

use std::thread;
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, Instant};

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::window::{KeyScancode, ModifiersState, MouseButton, VirtualKeyCode, WindowCreationOptions, WindowHandler, WindowHelper, WindowPosition, WindowSize, WindowStartupInfo};
use speedy2d::{Graphics2D, Window};

use ifmt::iformat;

use crate::editor::Editor;
use crate::animation::Animation;
use crate::editable::Editable;
use crate::editor::{EDITOR_OFFSET_TOP, EDITOR_PADDING};
use crate::menu_actions::MenuAction;
use crate::open_ai_wrapper::OpenAIWrapper;

const FPS: u64 = 60;
const FRAME_DURATION: u64 = 1000 / FPS; // ms

type MenuId = [isize; 3]; // Support 3 nested menu

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum FocusElement { Editor, Menu(MenuId), MenuInput(MenuId) }

#[derive(PartialEq, Debug, Clone)]
pub enum EditorEvent {
    Update,
    Redraw,
    Focus(FocusElement),
    MenuItemSelected(MenuAction),
    MenuItemUnselected(MenuAction, String),
    SetDirty(String, bool),
    LoadFile(String),
    OAIResponse(MenuId, Vec<String>)
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

    #[warn(unreachable_patterns)]
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
                MenuAction::NewFile(path) => self.editor.new_file(&path),
                MenuAction::Underline => self.editor.underline(),
                MenuAction::Bold => self.editor.bold(),
                MenuAction::Copy => self.editor.copy(),
                MenuAction::Cut => { self.editor.copy(); self.editor.delete_selection(); },
                MenuAction::Paste => self.editor.paste(),
                MenuAction::OpenSubMenu => {},
                MenuAction::CloseMenu => self.editor.menu.close(),
                MenuAction::FindAndJump(text) => self.editor.find(&text),
                MenuAction::AICorrect => OpenAIWrapper::correct(&self.editor.get_selected_text(), &self.editor.get_focus_menu().unwrap()),
                MenuAction::AIQuestion(question) => OpenAIWrapper::ask(&question.replace('$', &self.editor.get_selected_text()), &self.editor.get_focus_menu().unwrap()),
                MenuAction::ToggleLoader(id) => self.editor.get_menu(id).toggle_loader(),
                MenuAction::ReplaceSelection(string) => self.editor.add_text(&string),
                _ => {}
            },
            EditorEvent::MenuItemUnselected(_item, key) => self.editor.add_char(key),
            EditorEvent::LoadFile(path) => set_app_title(helper, &path),
            EditorEvent::SetDirty(path, is_dirty) => set_app_title(helper, &if !is_dirty { path } else { path + " *" }),
            EditorEvent::OAIResponse(menu_id, choices) => self.editor.get_menu(menu_id).async_callback(choices),
            _ => {}
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
        self.mouse_position = position;
        if self.mouse_button_pressed.0 || self.editor.modifiers.shift() {
            self.editor.camera.safe_zone_size = 5.;
            self.editor.update_selection(position);
            helper.request_redraw();
        } else {
            self.editor.camera.safe_zone_size = 30.;
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
            MouseButton::Right => {
                self.mouse_button_pressed.1 = true;
                self.editor.toggle_contextual_menu();
            },
            _ => {}
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
        if let Some(keycode) = virtual_key_code {
            match self.focus {
                FocusElement::Menu(id) => self.editor.get_menu(id).handle_key(keycode, modifiers),
                FocusElement::Editor => self.editor.handle_key(keycode),
                FocusElement::MenuInput(id) => self.editor.get_menu(id).send_key_to_input(keycode, modifiers),
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
                FocusElement::Menu(id) => {
                    // Cancel chip should disapear on keydown but the char should be added anyway
                    // Ugly
                    let menu = self.editor.get_menu(id);
                    if menu.items[0].action == MenuAction::CancelChip {
                        self.editor.add_char(unicode_codepoint.to_string());
                        self.editor.update_text_layout();
                    }
                }
            }
            helper.request_redraw();
        }
    }

    fn on_keyboard_modifiers_changed(&mut self, _helper: &mut WindowHelper<EditorEvent>, state: ModifiersState) { self.editor.modifiers = state.clone(); }
}

fn set_app_title(helper: &mut WindowHelper<EditorEvent>, path: &str) {
    let filename = Path::new(path).file_name().unwrap_or(OsStr::new("")).to_str().unwrap();
    helper.set_title(&iformat!("Text Editor - {filename}"))
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
