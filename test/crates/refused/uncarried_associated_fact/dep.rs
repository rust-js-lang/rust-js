//! A library trait whose associated type its own code asks nothing of, and
//! whose impl gives it as a type its contract can't say, a `dyn` beside its
//! parameter: its dictionaries carry no fact of it (ADR 0145, amended,
//! #476, #1420).
pub trait Holder {
    type Held;

    fn held(self) -> Self::Held;
}

pub struct Boxed<T>(pub T);

impl<T> Holder for Boxed<T> {
    type Held = (T, Box<dyn Fn(&T) -> usize>);

    fn held(self) -> Self::Held {
        (self.0, Box::new(|_| 1))
    }
}
