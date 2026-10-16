// A `MaybeUninit` of a type parameter that's an array, smallvec's
// `A: Array`, each of whose impls is an array (ADR 0417): an array of slots,
// none written yet, written an item at a time through a pointer to its
// first, and read back.
use std::mem::MaybeUninit;
use std::ptr;

/// What an array is, as smallvec's trait says it.
unsafe trait Array {
    type Item;
    fn size() -> usize;
}

unsafe impl<T, const N: usize> Array for [T; N] {
    type Item = T;
    fn size() -> usize {
        N
    }
}

/// Up to `A::size()` items, kept in place.
struct Inline<A: Array> {
    len: usize,
    data: MaybeUninit<A>,
}

impl<A: Array> Inline<A> {
    fn new() -> Self {
        Inline { len: 0, data: unsafe { MaybeUninit::uninit().assume_init() } }
    }

    fn push(&mut self, item: A::Item) -> bool {
        if self.len == A::size() {
            return false;
        }
        unsafe { ptr::write((self.data.as_mut_ptr() as *mut A::Item).add(self.len), item) };
        self.len += 1;
        true
    }

    fn get(&self, i: usize) -> Option<&A::Item> {
        (i < self.len).then(|| unsafe { &*(self.data.as_ptr() as *const A::Item).add(i) })
    }
}

fn main() {
    let mut words: Inline<[String; 2]> = Inline::new();
    for w in ["a", "b", "c"] {
        println!("{}", words.push(w.to_string()));
    }
    println!("{:?} {:?} {:?}", words.get(0), words.get(1), words.get(2));
    let empty: Inline<[u8; 3]> = Inline::new();
    println!("{} {:?}", empty.len, empty.get(0));
}
