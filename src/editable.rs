use speedy2d::dimen::Vector2;
use speedy2d::window::VirtualKeyCode;

pub trait Editable {
    fn add_char(&mut self, c: String);

    fn delete_char(&mut self);

    fn handle_key(&mut self, keycode: VirtualKeyCode);

    fn move_cursor(&mut self, position: Vector2<u32>);

    fn move_cursor_relative(&mut self, rel_x: i32, rel_y: i32);

    fn shortcut(&mut self, c: char);

    fn begin_selection(&mut self);

    fn end_selection(&mut self);

    fn update_selection(&mut self, position: Vector2<f32>);

    fn delete_selection(&mut self);

    fn get_mouse_position_index(&mut self, position: Vector2<f32>) -> Vector2<u32>;

    fn get_valid_cursor_position(&mut self, position: Vector2<u32>) -> Vector2<u32>;

    fn select_current_word(&mut self);

    fn select_all(&mut self);

    fn select_current_line(&mut self);

    fn copy(&mut self);

    fn paste(&mut self);
}