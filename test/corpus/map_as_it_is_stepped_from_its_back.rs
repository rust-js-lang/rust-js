//@ library-differs
// `next_back()` of a map made where it's stepped, `v.into_iter().map(|x|
// x).next_back()`: its source's last item, mapped, as Rust's
// `Map::next_back` gives it, in generic code and not, of items with a
// destructor, the rest dropped as the temporary is, and of `Option`s. A
// map giving each item as it is was refused (#1406).
use std::fmt::Debug;

#[derive(Debug, Clone)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn last_kept<T>(v: Vec<T>) -> Option<T> {
    v.into_iter().map(|x| x).next_back()
}

fn last_cloned<T: Clone>(v: &[T]) -> Option<T> {
    v.iter().map(|x| x.clone()).next_back()
}

fn last_wrapped<T>(v: Vec<T>) -> Option<(T, usize)> {
    v.into_iter().map(|x| (x, 1)).next_back()
}

fn last_shown<T: Debug>(v: Vec<T>) -> Option<String> {
    v.into_iter().map(|x| format!("<{x:?}>")).next_back()
}

fn last_byte(v: Vec<u8>) -> Option<u8> {
    v.into_iter().map(|x| x).next_back()
}

fn main() {
    println!("{:?} {:?}", last_kept(vec![1, 2, 3]), last_kept::<u8>(vec![]));
    println!("{:?}", last_kept(vec![Some(1), None]));
    println!("{:?}", last_kept(vec![None, Some('a')]));
    println!("{:?}", last_byte(vec![7, 8]));
    let got = last_kept(vec![Loud(1), Loud(2), Loud(3)]);
    println!("got {got:?}");
    drop(got);
    let louds = [Loud(10), Loud(11)];
    println!("{:?}", last_cloned(&louds));
    println!("{:?}", last_wrapped(vec!["a", "b"]));
    println!("{:?}", last_wrapped(vec![Loud(20), Loud(21)]));
    println!("{:?} {:?}", last_shown(vec!['x', 'y']), last_shown::<u8>(vec![]));
    println!("end");
}
