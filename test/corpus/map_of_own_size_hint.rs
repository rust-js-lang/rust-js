// A `map` of the crate's iterator with its own `size_hint`, made in place,
// held, given to generic code and peeked, hints and counts what its
// source does, as std's `Map` passes it on (#382).

struct Count(u32);
impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize))
    }
}
impl ExactSizeIterator for Count {}
fn hint<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}
fn main() {
    println!("{:?}", Count(3).map(|x| x * 2).size_hint());
    let held = Count(4).map(|x| x + 1);
    println!("{:?} {}", held.size_hint(), held.len());
    println!("{:?}", hint(Count(5).map(|x| x * 3)));
    let mut p = Count(2).map(|x| x).peekable();
    p.peek();
    println!("{:?} {}", p.size_hint(), p.len());
    let v: Vec<u32> = Count(6).map(|x| x * 10).collect();
    println!("{} {}", v.len(), v.capacity() >= 6);
}
