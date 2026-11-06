// An `extend` of a `Vec`, a `BinaryHeap` or a `VecDeque` asks its
// iterator's `size_hint` as std's does as the collection fills (#849):
// `extend_desugared` as it takes an item where the `Vec` is full, reserving
// its lower bound and one more, at least twice what it had; a `VecDeque`'s
// `spec_extend` at its first item too. Its capacity is native Rust's where
// it's made here, by `new`, `with_capacity`, `vec![..]` or a `TrustedLen`
// collect, and grows as std's does by `extend`, `reserve` and
// `reserve_exact`. Where asking it panics, the item taken drops first.
use std::collections::{BinaryHeap, VecDeque};

struct Loud {
    n: u32,
    panics_below: u32,
}

impl Iterator for Loud {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        Some(self.n)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        print!("[hint {}] ", self.n);
        if self.n < self.panics_below {
            panic!("hint");
        }
        (self.n as usize / 2, None)
    }
}

fn loud(n: u32) -> Loud {
    Loud { n, panics_below: 0 }
}

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        print!("(drop {}) ", self.0);
    }
}

fn main() {
    let mut v: Vec<u32> = Vec::new();
    v.extend(loud(9));
    println!("new {} {}", v.len(), v.capacity());
    let mut v: Vec<u32> = Vec::with_capacity(2);
    v.extend(loud(5));
    v.extend(loud(3));
    println!("with capacity {} {}", v.len(), v.capacity());
    let mut v = vec![1u32, 2];
    v.extend(loud(4));
    println!("literal {} {}", v.len(), v.capacity());
    let mut v = vec![0u8; 3];
    v.reserve(2);
    v.extend(loud(9).map(|x| x as u8));
    println!("bytes {} {}", v.len(), v.capacity());
    let mut v: Vec<u32> = (0..3).collect();
    v.reserve_exact(1);
    v.extend(loud(4));
    println!("collected {} {}", v.len(), v.capacity());
    let mut h: BinaryHeap<u32> = BinaryHeap::new();
    h.extend(loud(6));
    println!("heap {} {:?}", h.len(), h.peek());
    let mut d: VecDeque<u32> = VecDeque::with_capacity(10);
    d.extend(loud(4));
    d.extend(loud(9));
    println!("deque {} {}", d.len(), d.capacity());
    let mut units: Vec<()> = Vec::new();
    units.extend(loud(3).map(|_| ()));
    println!("units {}", units.len());
    let caught = std::panic::catch_unwind(|| {
        let mut v: Vec<Noisy> = Vec::new();
        v.extend(Loud { n: 9, panics_below: 5 }.map(Noisy));
        v.len()
    });
    println!("panicked {}", caught.is_err());
}
