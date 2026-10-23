//@ compile-fail: rust-js does not support a `&mut &mut Counter`, a `&mut` of a `&mut` of an object, given to generic code yet
// Generic code's each `&mut` is a box (ADR 0454), where a caller's `&mut`
// of an object is the object, adapted one level where it's given: a
// `&mut` of a `&mut` of one would be a box short, so it's refused (#167).
// A `&mut` of one, and of a `&mut` of a number, are given as they are.
trait Count {
    fn count(&mut self) -> u32;
}

struct Counter(u32);

impl Count for Counter {
    fn count(&mut self) -> u32 {
        self.0 += 1;
        self.0
    }
}

impl<T: Count + ?Sized> Count for &mut T {
    fn count(&mut self) -> u32 {
        (**self).count() * 10
    }
}

fn drive<K: Count>(mut k: K) -> u32 {
    k.count()
}

fn main() {
    let mut c = Counter(0);
    println!("{}", drive(&mut c));
    println!("{}", drive(&mut &mut c));
}
