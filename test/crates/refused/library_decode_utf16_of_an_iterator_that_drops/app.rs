//! A consumer giving the library's `decoded` an iterator with a `Drop`,
//! which std's `DecodeUtf16` drops at the loop's end: refused at its call,
//! as one crate's own generic code is, where the consumer's copy of
//! `decoded` (ADR 0914) dropped the runtime's `$decodeUtf16` iterator's
//! `iter`, which it hasn't, and threw (#2575).
#[derive(Clone, Debug)]
struct Loud<I> {
    it: I,
    name: &'static str,
}

impl<I: Iterator> Iterator for Loud<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        println!("next {}", self.name);
        self.it.next()
    }
}

impl<I> Drop for Loud<I> {
    fn drop(&mut self) {
        println!("drop {}", self.name);
    }
}

pub fn main() {
    dep::decoded(Loud {
        it: vec![0x61u16, 0xD800, 0x62].into_iter(),
        name: "own",
    });
    dep::decoded(dep::Source(vec![0x63u16].into_iter()));
}
