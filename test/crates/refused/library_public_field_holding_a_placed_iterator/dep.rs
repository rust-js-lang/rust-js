//! A library's holder of what it's given's iterator in a public field,
//! `Open<I>`, which its generic code makes, holding the iterator in a place
//! of its own where it's one JS holds as no object (#2255), and its
//! consumer may read by its shape.
pub struct Open<I> {
    pub inner: I,
}

pub fn open<I: Iterator>(mut inner: I) -> Open<I> {
    inner.next();
    Open { inner }
}
