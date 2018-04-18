use imgui::Ui;

pub trait ConstantEditor<T>: Default {
    /// Build editor for constant T and return true on change
    fn editor(&self, ui: &Ui, constant: &mut T) -> bool;
}
