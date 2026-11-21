// `Box::take(b)`, Rust 1.100's: what the box holds, moved out, and the box
// left unwritten, which `Box::write` or a `MaybeUninit`'s `write` fills
// again; what's taken dropped where it's left, as native's (#1656).
use std::mem::MaybeUninit;
struct L(u32);
impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let b = Box::new(String::from("hi"));
    let (s, empty) = Box::take(b);
    println!("{s}");
    let again: Box<String> = Box::write(empty, String::from("again"));
    println!("{again}");
    let (n, slot) = Box::take(Box::new(5u32));
    let refilled = Box::write(slot, n + 1);
    println!("{n} {refilled}");
    let (l, _slot) = Box::take(Box::new(L(1)));
    println!("took {}", l.0);
    drop(l);
    let (arr, mut slot2) = Box::take(Box::new([1u8, 2, 3]));
    slot2.write([9, 9, 9]);
    let back = unsafe { slot2.assume_init() };
    println!("{arr:?} {back:?}");
    let _unused: Box<MaybeUninit<L>> = Box::take(Box::new(L(2))).1;
    println!("end");
}
