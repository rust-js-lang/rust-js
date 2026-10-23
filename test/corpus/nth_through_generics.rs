// `nth` of an iterator with a destructor, or of items with one, given to
// generic code by value or through a `&mut`: it drops each item it passes
// over, and the function the iterator's (#180).
struct Plain(&'static str, u32);
impl Drop for Plain {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
impl Iterator for Plain {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.1 < 3 {
            self.1 += 1;
            Some(self.1)
        } else {
            None
        }
    }
}
struct Loud(u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop item {}", self.0);
    }
}
fn second<I: Iterator<Item = u32>>(mut items: I) -> Option<u32> {
    items.nth(1)
}
fn second_of<I: Iterator>(mut items: I) -> Option<I::Item> {
    items.nth(1)
}
fn third_ref<I: Iterator<Item = u32>>(items: &mut I) -> Option<u32> {
    items.nth(2)
}
fn main() {
    println!("{:?}", second(Plain("a", 0)));
    println!("{:?}", second(vec![1u32, 2, 3].into_iter()));
    let got = second_of(vec![Loud(1), Loud(2), Loud(3)].into_iter());
    println!("got {:?}", got.as_ref().map(|l| l.0));
    drop(got);
    let mut p = Plain("b", 0);
    println!("{:?}", third_ref(&mut p));
    println!("{:?}", p.next());
    println!("end");
}
