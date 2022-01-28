use std::fmt;

pub(crate) type MenuActionFn = fn(String) -> MenuAction;

#[derive(PartialEq, Debug, Clone)]
pub enum MenuAction {
    Open(String),
    OpenWithInput(String),
    Save(String),
    SaveWithInput(String),
    Void,
    Information,
    Exit,
    CancelChip,
    Underline,
    Copy,
    Cut,
    Paste,
    Bold,
    OpenSubMenu,
    CloseMenu,
    PrintWithInput,
    Print(String),
    NewFile(String),
    NewFileWithInput(String),
    FindAndJumpWithInput,
    FindAndJump(String)
}

impl fmt::Display for MenuAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{:?}", self) }
}

impl MenuAction {
    pub fn get_fn(action: &MenuAction) -> MenuActionFn {
        match action {
            MenuAction::OpenWithInput(_) => MenuAction::Open,
            MenuAction::SaveWithInput(_) => MenuAction::Save,
            MenuAction::PrintWithInput => MenuAction::Print,
            MenuAction::NewFileWithInput(_) => MenuAction::NewFile,
            MenuAction::FindAndJumpWithInput => MenuAction::FindAndJump,
            _ => MenuAction::Print
        }
    }
}