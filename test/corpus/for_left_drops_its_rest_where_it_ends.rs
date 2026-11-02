// A `for` over a `Vec`'s `into_iter()`, in another loop, drops what it
// didn't step where it ends, by its `break` or as it runs out, before the
// statement after it runs: never after that statement, as a `finally`
// around both would (#465, the Nightly's seed 12455721). A plain value
// kept after it may run before the drop: nothing the drop does sees it;
// a field a drop reads, set after it, is set after the drop.
use std::cell::Cell;
use std::rc::Rc;

struct D(u32);

struct Seen {
    n: Cell<u32>,
}

struct Reads(Rc<Seen>, u32);

impl Drop for Reads {
    fn drop(&mut self) {
        println!("drop {} sees {}", self.1, self.0.n.get());
    }
}

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn id<T>(x: T) -> T {
    x
}

fn main() {
    let mut round: u32 = 0;
    let mut last = 0;
    while round < id(3u32) {
        round += 1;
        for d in vec![D(id(14u32)), D(id(15u32)), D(id(16u32)), D(id(17u32))].into_iter() {
            println!("got {}", d.0);
            if round != 2 && d.0 == 16 {
                break;
            }
        }
        println!("{:?} after round {}", Vec::<i32>::new().is_empty(), round);
    }
    while last < 2 {
        last += 1;
        for d in vec![D(20), D(21), D(22)] {
            if d.0 == 21 {
                break;
            }
        }
        round = last;
    }
    let seen = Rc::new(Seen { n: Cell::new(0) });
    while seen.n.get() < 4 {
        for r in vec![Reads(Rc::clone(&seen), 1), Reads(Rc::clone(&seen), 2)] {
            if r.1 == 1 {
                break;
            }
        }
        seen.n.set(seen.n.get() + 2);
    }
    println!("end {} {} {}", round, last, seen.n.get());
}
