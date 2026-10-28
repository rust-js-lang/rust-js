// An enumeration of items with a destructor stepped from its back: one
// held in a variable, by its own `next_back()`, and one made in place
// beside it, of the same type, by `rev()`, which the held one's drop puts
// in a variable of its own. Each made knowing its back and length, its
// index from the back where it is, and what it has left dropped when it
// is (#383).
struct Loud(u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let v = vec![Loud(1), Loud(2), Loud(3)];
    let mut it = v.into_iter().fuse().enumerate();
    let last = it.next_back().map(|(i, l)| (i, l.0));
    println!("last {last:?}");
    let first = it.next().map(|(i, l)| (i, l.0));
    println!("first {first:?}");
    let w = vec![4u32, 5, 6];
    let mut e = w.iter().fuse().enumerate();
    println!("{:?} {:?}", e.next_back(), e.next());
    let r: Vec<_> = vec![Loud(7), Loud(8)]
        .into_iter()
        .fuse()
        .enumerate()
        .rev()
        .map(|(i, l)| (i, l.0))
        .collect();
    println!("{r:?}");
}
