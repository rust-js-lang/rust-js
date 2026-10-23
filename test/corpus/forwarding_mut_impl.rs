// `impl<T: Trait + ?Sized> Trait for &mut T`, the forwarding impl std
// writes for `Write` and `Iterator`: `(**self).m()` is given the cell its
// `&mut T` is, so what the inner method writes, of a number or a
// `String`, is written there.
trait Inc {
    fn inc(&mut self) -> i32;
}

impl Inc for i32 {
    fn inc(&mut self) -> i32 {
        *self += 1;
        *self
    }
}

impl<T: Inc + ?Sized> Inc for &mut T {
    fn inc(&mut self) -> i32 {
        (**self).inc()
    }
}

trait Sink {
    fn put(&mut self, x: u8);
}

impl Sink for String {
    fn put(&mut self, x: u8) {
        self.push(char::from(b'0' + x));
    }
}

impl Sink for Vec<u8> {
    fn put(&mut self, x: u8) {
        self.push(x);
    }
}

impl<S: Sink + ?Sized> Sink for &mut S {
    fn put(&mut self, x: u8) {
        (**self).put(x);
    }
}

fn twice<I: Inc>(mut i: I) -> i32 {
    i.inc();
    i.inc()
}

fn emit<S: Sink>(mut s: S) {
    s.put(1);
    s.put(2);
}

fn main() {
    let mut n = 0;
    println!("{} {n}", twice(&mut n));
    let mut text = String::new();
    let mut bytes = Vec::new();
    emit(&mut text);
    emit(&mut bytes);
    println!("{text:?} {bytes:?}");
}
