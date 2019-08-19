use std::error;

use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;

use imgui::{Clipboard, ImGui};

pub fn setup(imgui: &mut ImGui) -> Result<(), Box<dyn error::Error>> {
    let ctx: ClipboardContext = ClipboardProvider::new()?;
    imgui.prepare_clipboard::<ClipboardImpl>(ctx);
    Ok(())
}

struct ClipboardImpl;

impl Clipboard for ClipboardImpl {
    type UserData = ClipboardContext;
    fn get_clipboard_text(user_data: &mut Self::UserData) -> String {
        user_data.get_contents().unwrap_or_else(|e| {
            eprintln!("Failed to get clipboard: {:?}", e);
            String::new()
        })
    }
    fn set_clipboard_text(user_data: &mut Self::UserData, text: String) {
        if let Err(e) = user_data.set_contents(text) {
            eprintln!("Failed to set clipboard: {:?}", e);
        }
    }
}
