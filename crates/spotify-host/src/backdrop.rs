//! Which system backdrop to use for the host window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backdrop {
    None,
    Mica,
    Acrylic,
}

impl Default for Backdrop {
    fn default() -> Self {
        Backdrop::Mica
    }
}
