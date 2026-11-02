//! A library trait whose associated type its own code asks nothing of: its
//! dictionaries carry no fact of it (ADR 0145, amended, #476).
pub trait Holder {
    type Held;

    fn held(&self) -> &Self::Held;
}

pub struct Boxed<T>(pub T);

impl<T> Holder for Boxed<T> {
    type Held = T;

    fn held(&self) -> &T {
        &self.0
    }
}
