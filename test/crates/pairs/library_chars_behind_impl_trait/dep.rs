//! A library's `chars()` behind an `impl DoubleEndedIterator`, which
//! another crate steps from its back (#1186): the library never steps it,
//! but makes it the runtime's, which knows its back.
pub fn letters(s: &str) -> impl DoubleEndedIterator<Item = char> + '_ {
    s.chars()
}
