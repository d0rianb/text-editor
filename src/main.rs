#[macro_use]
extern crate derivative;

mod animation;
mod cursor;
mod editor;
mod font;
mod line;

use std::thread;
use std::env;
use std::time::{Duration, Instant};

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::window::{KeyScancode, ModifiersState, MouseButton, VirtualKeyCode, WindowCreationOptions, WindowHandler, WindowHelper, WindowPosition, WindowSize, WindowStartupInfo};
use speedy2d::{Graphics2D, Window};

use editor::Editor;

const FPS: u64 = 60;
const FRAME_DURATION: u64 = 1000 / FPS; // ms

#[derive(PartialEq, Debug, Clone, Copy)]
enum EditorEvent { Udpate, Redraw }

struct EditorWindowHandler {
    editor: Editor,
    last_editor_size: Vector2<u32>,
    tick_timestamp: Instant,
    modifiers : ModifiersState
}

impl WindowHandler<EditorEvent> for EditorWindowHandler {
    fn on_start(&mut self, helper: &mut WindowHelper<EditorEvent>, _info: WindowStartupInfo) {
        let event_sender = helper.create_user_event_sender();
        self.editor.event_sender = Some(event_sender.clone());
        self.editor.cursor.set_animation_event_sender(Some(event_sender.clone()));
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

    fn on_draw(&mut self, helper: &mut WindowHelper<EditorEvent>, graphics: &mut Graphics2D) {
        graphics.clear_screen(Color::WHITE);
        self.editor.render(graphics);
    }

    fn on_mouse_button_down(&mut self, helper: &mut WindowHelper<EditorEvent>, _button: MouseButton) {
        helper.request_redraw();
    }

    fn on_key_down(&mut self, helper: &mut WindowHelper<EditorEvent>, virtual_key_code: Option<VirtualKeyCode>, _scancode: KeyScancode) {
        match virtual_key_code {
            Some(VirtualKeyCode::Right) => self.editor.move_cursor_relative(1, 0),
            Some(VirtualKeyCode::Left) => self.editor.move_cursor_relative(-1, 0),
            Some(VirtualKeyCode::Up) => self.editor.move_cursor_relative(0, -1),
            Some(VirtualKeyCode::Down) => self.editor.move_cursor_relative(0, 1),
            _ => (),
        }
        helper.request_redraw();
    }

    fn on_keyboard_char(&mut self, _helper: &mut WindowHelper<EditorEvent>, unicode_codepoint: char) {
        match unicode_codepoint {
            '\u{7f}' | '\u{8}' => self.editor.delete_char(),
            '\r' => self.editor.new_line(),
            _ => {
                if self.modifiers.logo() { // Cmd key
                    self.editor.shortcut(unicode_codepoint);
                } else {
                    self.editor.add_char(unicode_codepoint.to_string())
                }
            },
        }
        self.editor.update_text_layout();
    }

    fn on_keyboard_modifiers_changed(&mut self, _helper: &mut WindowHelper<EditorEvent>, state: ModifiersState) {
        self.modifiers = state;
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let window = Window::new_with_user_events(
        "Editor",
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
    window.run_loop(EditorWindowHandler {
        editor,
        modifiers: ModifiersState::default(),
        last_editor_size: (1200, 800).into(),
        tick_timestamp: Instant::now(),
    });
}
