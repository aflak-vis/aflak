use imgui::{ImId, Ui};

/// Trait to define how to draw constant editor.
///
/// Constant values may be edited in the node editor.
/// Implementing this trait is necessary do define how the values flowing around
/// in the node editor can be manually edited.
pub trait ConstantEditor<T>: Default {
    /// Build editor for constant T and return new value if value changed
    fn editor<'a, I>(&self, ui: &Ui, constant: &T, id: I, read_only: bool) -> Option<T>
    where
        I: Into<ImId<'a>>;
}
