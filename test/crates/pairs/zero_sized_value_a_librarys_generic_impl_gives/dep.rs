// A library's generic impls that give a value of no size: a unit struct, an
// empty struct, an empty array and a tuple of them. Its consumer compiles
// each instance from the library's optimized MIR, where rustc drops a write
// of no size, `_0 = Refused`, so what's returned was a name nothing
// declared, and the JS was refused (#1889). It's the type's one value.
pub struct Refused;

#[derive(Debug)]
pub struct Empty {}

impl FromIterator<u32> for Refused {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let mut it = items.into_iter();
        if it.next() == Some(1) {
            panic!("refused");
        }
        Refused
    }
}

impl FromIterator<u32> for Empty {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        println!("empty of {}", items.into_iter().count());
        Empty {}
    }
}

pub fn nothing<T: Default>(_: T) -> ([u8; 0], (), Empty) {
    println!("nothing");
    ([], (), Empty {})
}

pub fn collected<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}
