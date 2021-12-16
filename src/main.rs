mod editor;
mod cursor;
mod font;
use editor::Editor;

use speedy2d::color::Color;
use speedy2d::{Graphics2D, Window};
use speedy2d::dimen::Vector2;
use speedy2d::window::{KeyScancode, ModifiersState, MouseButton, VirtualKeyCode, WindowHandler, WindowHelper, WindowStartupInfo};

struct EditorMyWindowHandler {
    editor: Editor
}

impl WindowHandler for EditorMyWindowHandler {

    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        graphics.clear_screen(Color::WHITE);
        self.editor.render(graphics);
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
            '\r' => (),
            _ => self.editor.add_char(unicode_codepoint.to_string())
        }
        self.editor.update();
    }

    fn on_keyboard_modifiers_changed(&mut self, helper: &mut WindowHelper, state: ModifiersState) {
    }
}

fn main() {
    let window = Window::new_centered("Editor", (1200, 800)).unwrap();
    let editor = Editor::new();
    window.run_loop(EditorMyWindowHandler { editor });
}