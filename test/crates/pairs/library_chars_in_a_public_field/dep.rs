//! A library's `chars()` in its struct's public field, which another
//! crate steps from its back (#1186): the library steps it only from its
//! front, but makes it the runtime's, which knows its back.
pub struct Cursor<'a> {
    pub chars: std::str::Chars<'a>,
    pub read: usize,
}

pub fn cursor(s: &str) -> Cursor<'_> {
    let mut chars = s.chars();
    let read = usize::from(chars.next().is_some());
    Cursor { chars, read }
}
