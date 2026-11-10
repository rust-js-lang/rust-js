// A boxed closure given by value to a std consumer, `all`, `any`, `fold`,
// `for_each`, `scan`, `or_insert_with` or `catch_unwind`, is dropped where
// the consumer's done with it, as a closure is (#1220).
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let b = Rc::new(Cell::new(2));
    let l = Loud(1);
    let f: Box<dyn FnMut(&u32) -> bool> = Box::new(move |&x| {
        let _ = &l;
        x < 5
    });
    println!("all {}", [0, 3, 9].iter().all(f));
    let l = Loud(2);
    let c = b.clone();
    let g: Box<dyn FnMut(&u32) -> bool> = Box::new(move |&x| {
        let _ = &l;
        x < c.get()
    });
    println!("any {}", [0, 3].iter().any(g));
    let l = Loud(3);
    let h: Box<dyn FnMut(u32, &u32) -> u32> = Box::new(move |a, &x| {
        let _ = &l;
        a + x
    });
    println!("fold {}", [4, 5].iter().fold(0, h));
    let l = Loud(4);
    let p: Box<dyn FnMut(&u32)> = Box::new(move |x| {
        let _ = &l;
        println!("each {x}");
    });
    [7, 8].iter().for_each(p);
    let l = Loud(5);
    let s: Box<dyn FnMut(&mut u32, &u32) -> Option<u32>> = Box::new(move |acc, &x| {
        let _ = &l;
        *acc += x;
        Some(*acc)
    });
    let sums: Vec<u32> = [1, 2, 3].iter().scan(0, s).collect();
    println!("scan {sums:?}");
    let l = Loud(6);
    let k: Box<dyn FnOnce() -> u32> = Box::new(move || {
        let _ = &l;
        42
    });
    let mut m: HashMap<u32, u32> = HashMap::new();
    println!("entry {}", m.entry(1).or_insert_with(k));
    let l = Loud(7);
    let u: Box<dyn FnOnce() -> u32 + std::panic::UnwindSafe> = Box::new(move || {
        let _ = &l;
        7
    });
    println!("caught {:?}", std::panic::catch_unwind(u).ok());
    // And as a panic in the consumer unwinds.
    let l = Loud(8);
    let q: Box<dyn FnMut(&u32) -> bool> = Box::new(move |&x| {
        let _ = &l;
        if x == 3 {
            panic!("three");
        }
        x > 5
    });
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| [0, 3].iter().any(q)));
    println!("unwound {}", r.is_err());
    println!("end");
}
