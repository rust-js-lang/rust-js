//! A string's `chars()` made by a library and given to another crate, or
//! given by it to the library's generic code (#1186): each crate may ask
//! it its back, its rest or its hint, which only one crate sees.
use std::str::Chars;

pub fn letters(s: &str) -> Chars<'_> {
    s.chars()
}

pub fn indexed(s: &str) -> std::str::CharIndices<'_> {
    s.char_indices()
}

pub fn opaque(s: &str) -> impl DoubleEndedIterator<Item = char> + '_ {
    s.chars()
}

pub struct Cursor<'a> {
    pub chars: Chars<'a>,
}

pub fn cursor(s: &str) -> Cursor<'_> {
    Cursor { chars: s.chars() }
}

pub fn hint<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

pub fn last<I: DoubleEndedIterator<Item = char>>(mut it: I) -> Option<char> {
    it.next_back()
}

pub fn first(s: &str) -> Option<char> {
    s.chars().next()
}
