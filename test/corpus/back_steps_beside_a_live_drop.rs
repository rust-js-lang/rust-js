// An `enumerate()`, a `skip()` or a `take()` stepped from its back, made
// in a function where something with a destructor is live, or after a
// peeked `peekable()` was counted, reads as it does where nothing is: each
// call of the chain unwinds to the one cleanup that drops what's live, so
// what's made between them stays made in place, not held in a variable no
// form of it is left to read (#723). Of a `Vec`'s, an array's, a slice's
// and a range's items, beside a live `D`, and as a panic in a closure
// unwinds past them, dropping it.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn f(x: u32) -> u32 {
    x * 10
}

fn beside_a_live_drop() {
    let _g = D(0);
    let v: Vec<(usize, u32)> = vec![1, 2, 3, 4].into_iter().enumerate().rev().collect();
    println!("{v:?}");
    let v: Vec<(usize, u32)> = vec![1, 2, 3, 4].into_iter().skip(1).enumerate().rev().collect();
    println!("{v:?}");
    let v: Vec<(usize, u32)> = vec![1, 2, 3, 4].into_iter().take(3).enumerate().rev().collect();
    println!("{v:?}");
    let v: Vec<u32> = vec![1, 2, 3, 4].into_iter().skip(1).rev().collect();
    println!("{v:?}");
    let v: Vec<u32> = vec![1, 2, 3, 4].into_iter().take(3).rev().collect();
    println!("{v:?}");
    let v: Vec<(usize, u32)> = vec![1, 2, 3, 4, 5].into_iter().skip(1).take(3).enumerate().rev().collect();
    println!("{v:?}");
    let v: Vec<(usize, u32)> = vec![1, 2, 3, 4].into_iter().enumerate().skip(1).rev().collect();
    println!("{v:?}");
    let v: Vec<(usize, u32)> = [5u32, 6, 7].into_iter().skip(1).enumerate().rev().collect();
    println!("{v:?}");
    let v: Vec<(usize, &u32)> = [5u32, 6, 7, 8][1..].iter().skip(1).enumerate().rev().collect();
    println!("{v:?}");
    let v: Vec<(usize, u32)> = (1..5u32).enumerate().rev().collect();
    println!("{v:?}");
    let v: Vec<(usize, u32)> = (1..9u32).skip(2).enumerate().rev().collect();
    println!("{v:?}");
    let v: Vec<(usize, u32)> = (1..9u32).take(3).enumerate().rev().collect();
    println!("{v:?}");
    let v: Vec<(usize, u32)> = (1..5u32).map(f).enumerate().rev().collect();
    println!("{v:?}");
    let v: Vec<(usize, u16)> = (1..=5u16).enumerate().rev().collect();
    println!("{v:?}");
    println!("{:?}", vec![1u32, 2, 3, 4].into_iter().enumerate().rev().nth(2));
    println!("{:?}", vec![1u32, 2, 3, 4].into_iter().enumerate().next_back());
    println!("{:?}", vec![D(1), D(2), D(3)].into_iter().enumerate().rev().map(|(i, d)| (i, d.0)).collect::<Vec<_>>());
}

fn after_a_peeked_count() {
    let mut it = vec![D(10)].into_iter().peekable();
    it.peek();
    println!("count {}", it.enumerate().count());
    let v: Vec<(usize, u32)> = vec![1, 2, 3, 4].into_iter().skip(1).enumerate().rev().collect();
    println!("{v:?}");
    let v: Vec<u32> = vec![1, 2, 3, 4].into_iter().take(3).rev().collect();
    println!("{v:?}");
    let v: Vec<(usize, u32)> = (1..9u32).skip(2).enumerate().rev().collect();
    println!("{v:?}");
}

fn main() {
    beside_a_live_drop();
    after_a_peeked_count();
    // A panic in a closure between them unwinds through the one cleanup,
    // dropping what's live once.
    let r = catch_unwind(AssertUnwindSafe(|| {
        let _g = D(20);
        let v: Vec<(usize, u32)> = vec![1, 2, 3, 4]
            .into_iter()
            .map(|x| {
                assert!(x != 2, "two");
                x
            })
            .skip(1)
            .enumerate()
            .rev()
            .collect();
        println!("{v:?}");
    }));
    println!("panicked {}", r.is_err());
}
