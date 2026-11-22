//! A library trait's defaults that make std's values of no size, as
//! quickcheck's `Arbitrary::shrink` makes an `iter::Empty`, which a
//! consumer's impl compiles itself (ADR 0914): its optimized MIR has each as
//! a constant of no size, of a type parameter's items or of `u8`'s, and an
//! `io::Empty`.
pub trait Shrink: Sized + Clone + 'static {
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(std::iter::empty())
    }

    fn bytes(&self) -> Box<dyn Iterator<Item = u8>> {
        Box::new(std::iter::empty())
    }

    fn reader(&self) -> Box<dyn std::io::Read> {
        Box::new(std::io::empty())
    }
}
