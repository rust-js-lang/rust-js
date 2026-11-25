//! A library's generic code that decodes what its caller gives as UTF-16,
//! `char::decode_utf16`, whose iterator std's struct holds and drops at the
//! loop's end (#2575).
pub struct Source<I>(pub I);

impl<I: Iterator> Iterator for Source<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        println!("next");
        self.0.next()
    }
}

impl<I> Drop for Source<I> {
    fn drop(&mut self) {
        println!("drop source");
    }
}

pub fn decoded<I: Iterator<Item = u16>>(it: I) {
    for r in char::decode_utf16(it) {
        println!("{r:?}");
    }
}
