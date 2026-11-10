// A key function given boxed, `Box<dyn FnMut>`, to `max_by_key`,
// `min_by_key` and `is_sorted_by_key`, and through a `&mut` of a closure:
// the key's type is what the function gives an item, `<F as
// FnOnce<(&T,)>>::Output`, which a box names as a closure does (#1223). A
// key with a destructor is dropped where std drops it, and the box itself
// as the consumer returns.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Key(i32);

impl Drop for Key {
    fn drop(&mut self) {
        println!("drop key {}", self.0);
    }
}

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let words = ["pear", "fig", "banana", "kiwi"];

    let by_len: Box<dyn FnMut(&&&str) -> usize> = Box::new(|w| w.len());
    println!("{:?}", words.iter().max_by_key(by_len));
    let negated: Box<dyn FnMut(&&i32) -> i32> = Box::new(|x| -**x);
    println!("{:?}", [1, 2, 3].iter().max_by_key(negated));
    let last: Box<dyn FnMut(&char) -> u32> = Box::new(|c| *c as u32);
    println!("{:?}", "rust".chars().min_by_key(last));
    let sorted: Box<dyn FnMut(i32) -> i32> = Box::new(|x| x / 10);
    println!("{}", [3, 12, 15, 40].iter().copied().is_sorted_by_key(sorted));
    let unsorted: Box<dyn Fn(&&str) -> usize> = Box::new(|w| w.len());
    println!("{}", words.iter().is_sorted_by_key(unsorted));

    let mut calls = 0;
    let mut counted = |w: &&&str| {
        calls += 1;
        w.as_bytes()[0]
    };
    println!("{:?}", words.iter().min_by_key(&mut counted));
    println!("{calls} calls");

    let dropping: Box<dyn FnMut(&i32) -> Key> = Box::new(|x| Key(x % 7));
    println!("{:?}", [5, 9, 13, 2].into_iter().max_by_key(dropping));
    let dropping: Box<dyn FnMut(&&i32) -> Key> = Box::new(|x| Key(**x));
    println!("{:?}", [8, 3, 6].iter().min_by_key(dropping));
    let held = Noisy("the key's box");
    let dropping: Box<dyn FnMut(&&i32) -> i32> = Box::new(move |x| {
        let _ = &held;
        **x % 4
    });
    println!("{:?}", [8, 3, 6].iter().max_by_key(dropping));
    println!("end");
}
