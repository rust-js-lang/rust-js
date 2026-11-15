// A library's `Display` that calls its trait's default, which takes the
// `Formatter`, of an impl that doesn't write it: a struct's, a const generic
// one's and a type parameter's. The library frames each writer to write
// through a sink, and called the default by its own function, which nothing
// writes where its impls copy it, so its JS was refused (#1889). It's the
// impl's copy, its dictionary's.
use std::fmt;

pub trait Measured {
    fn written(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    fn shown(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<")?;
        self.written(f)?;
        write!(f, ">")
    }
}

pub struct Plain(pub u32);

pub struct Buf<const SIZE: usize> {
    pub bytes: [u8; SIZE],
}

pub struct Items<T>(pub Vec<T>);

impl Measured for Plain {
    fn written(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "plain {}", self.0)
    }
}

impl<const SIZE: usize> Measured for Buf<SIZE> {
    fn written(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} of {}", self.bytes.len(), SIZE)
    }
}

impl<T: fmt::Debug> Measured for Items<T> {
    fn written(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl fmt::Display for Plain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.shown(f)
    }
}

impl<const SIZE: usize> fmt::Display for Buf<SIZE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.shown(f)
    }
}

impl<T: fmt::Debug> fmt::Display for Items<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.shown(f)
    }
}
