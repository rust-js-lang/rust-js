// A `Peekable` of what has a destructor is closed as Rust drops one: its
// source first, what it hasn't given, then what it peeked at; and its
// `len()` and `next_back()` count and give what it peeked at too (ADR
// 0514).
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn more() {
    let mut it = vec![Loud(1), Loud(2)].into_iter().peekable();
    println!("len {}", it.len());
    it.peek();
    println!("len {}", it.len());
    drop(it);
}
fn main() {
    let mut it = vec![Loud(1), Loud(2), Loud(3)].into_iter().peekable();
    it.peek();
    println!("peeked");
    drop(it);
    let mut it = vec![Loud(4), Loud(5), Loud(6)].into_iter().peekable();
    let none = it.next_if(|l| l.0 == 9);
    println!("next_if {}", none.is_none());
    drop(it);
    let mut lazy = (7..10).map(Loud).peekable();
    lazy.peek();
    drop(lazy);
    let mut back = vec![Loud(20), Loud(21)].into_iter().peekable();
    back.peek();
    println!("{:?}", back.next_back().map(|l| l.0));
    println!("{:?}", back.next_back().map(|l| l.0));
    println!("{:?}", back.next_back().map(|l| l.0));
    more();
    println!("end");
}
