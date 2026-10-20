// `iter::repeat_n(x, n)` gives `n - 1` clones and then `x` itself, drops
// `x` at once of a count of none, and when it's dropped before its last;
// and a `Peekable`'s `peek_mut` is a `&mut` to the item it gives next, of
// an array's iterator and of a lazy one (ADR 0513).
#[derive(Debug)]
struct Loud(u8);
impl Clone for Loud {
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        Loud(self.0 + 10)
    }
}
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn repeated() {
    let v: Vec<u8> = std::iter::repeat_n(7, 3).collect();
    let s: String = std::iter::repeat_n("ab", 2).collect();
    println!("{v:?} {s}");
    let all: Vec<Loud> = std::iter::repeat_n(Loud(1), 2).collect();
    println!("all {}", all.len());
    let none = std::iter::repeat_n(Loud(2), 0);
    println!("none made");
    drop(none);
    let mut some = std::iter::repeat_n(Loud(3), 3);
    let first = some.next();
    println!("first {:?}", first.as_ref().map(|l| l.0));
    drop(some);
    println!("dropped");
}
fn peeked() {
    let mut it = vec![1, 2, 3].into_iter().peekable();
    if let Some(x) = it.peek_mut() {
        *x *= 10;
    }
    println!("{:?}", it.collect::<Vec<_>>());
    let mut lazy = (1..4).map(|x| x * 2).peekable();
    if let Some(x) = lazy.peek_mut() {
        *x += 100;
    }
    println!("{:?}", lazy.collect::<Vec<_>>());
    let mut words = vec![String::from("a"), String::from("b")].into_iter().peekable();
    if let Some(w) = words.peek_mut() {
        w.push('!');
    }
    println!("{:?}", words.collect::<Vec<_>>());
    let mut empty = Vec::<u8>::new().into_iter().peekable();
    println!("{:?}", empty.peek_mut());
}
fn main() {
    repeated();
    peeked();
    println!("end");
}
