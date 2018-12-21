use imgui::{ImId, Ui};

pub trait ConstantEditor<T>: Default {
    /// Build editor for constant T and return new value if value changed
    fn editor<'a, I>(&self, ui: &Ui, constant: &T, id: I, read_only: bool) -> Option<T>
    where
        I: Into<ImId<'a>>;
}
