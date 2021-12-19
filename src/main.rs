mod editor;
mod cursor;
mod font;
mod line;

use editor::Editor;

use speedy2d::color::Color;
use speedy2d::{Graphics2D, Window};
use speedy2d::dimen::Vector2;
use speedy2d::window::{KeyScancode, ModifiersState, MouseButton, VirtualKeyCode, WindowHandler, WindowHelper, WindowStartupInfo};

struct EditorWindowHandler {
    editor: Editor,
    last_editor_size: Vector2<u32>,
}

impl WindowHandler for EditorWindowHandler {
    fn on_start(&mut self, helper: &mut WindowHelper, info: WindowStartupInfo) {
        helper.request_redraw();
    }

    fn on_resize(&mut self, helper: &mut WindowHelper, size_pixels: Vector2<u32>) {
        if self.last_editor_size != size_pixels {
            self.editor.on_resize(size_pixels);
            self.editor.update_text_layout();
        }
        self.last_editor_size = size_pixels;
    }

    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        graphics.clear_screen(Color::WHITE);
        self.editor.render(graphics);
    }

    fn on_mouse_button_down(&mut self, helper: &mut WindowHelper, button: MouseButton) {
        helper.request_redraw();
    }

    fn on_key_down(&mut self, helper: &mut WindowHelper, virtual_key_code: Option<VirtualKeyCode>, scancode: KeyScancode) {
        match virtual_key_code {
            Some(VirtualKeyCode::Right) => self.editor.move_cursor_relative(1, 0),
            Some(VirtualKeyCode::Left) => self.editor.move_cursor_relative(-1, 0),
            _ => ()
        }
    }

    fn on_keyboard_char(&mut self, helper: &mut WindowHelper, unicode_codepoint: char) {
        match unicode_codepoint {
            '\u{7f}' =>  self.editor.delete_char(),
            '\t' => (),
            '\r' => self.editor.new_line(),
            _ => self.editor.add_char(unicode_codepoint.to_string())
        }
        self.editor.update_text_layout();
        helper.request_redraw();
    }

    fn on_keyboard_modifiers_changed(&mut self, helper: &mut WindowHelper, state: ModifiersState) {
    }
}

fn main() {
    let window = Window::new_centered("Editor", (1200, 800)).unwrap();
    let editor = Editor::new(1200., 800.);
    window.run_loop(EditorWindowHandler {
        editor,
        last_editor_size: (1200, 800).into(),
    });
}