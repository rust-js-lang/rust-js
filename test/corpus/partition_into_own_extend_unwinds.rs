// `partition` into the crate's own `Extend`, std's compiled body: of the
// crate's own iterator too, given to it as a JS iterator; and as a panic in
// its predicate or in a side's `extend` unwinds, what std drops, in its
// order, the item, what's left of what it steps, then the right side and
// the left (#2387, #2378).
use std::panic;

#[derive(Debug)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Default, Debug)]
struct Pile(Vec<u32>);

impl Drop for Pile {
    fn drop(&mut self) {
        println!("drop pile {:?}", self.0);
    }
}

impl Extend<Loud> for Pile {
    fn extend<I: IntoIterator<Item = Loud>>(&mut self, items: I) {
        for item in items {
            if item.0 == 7 {
                panic!("seven");
            }
            println!("extend {}", item.0);
            self.0.push(item.0);
        }
    }
}

struct Up(u32);

impl Iterator for Up {
    type Item = Loud;
    fn next(&mut self) -> Option<Loud> {
        self.0 += 1;
        println!("next {}", self.0);
        (self.0 <= 3).then(|| Loud(self.0))
    }
}

fn main() {
    let (odd, even): (Pile, Pile) = Up(0).partition(|l| l.0 % 2 == 1);
    println!("{:?} {:?}", odd.0, even.0);
    let in_f = panic::catch_unwind(|| {
        let items = vec![Loud(1), Loud(2), Loud(3), Loud(4)];
        let _: (Pile, Pile) = items.into_iter().partition(|l| {
            if l.0 == 3 {
                panic!("three");
            }
            l.0 % 2 == 0
        });
    });
    println!("in f: {}", in_f.is_err());
    let in_extend = panic::catch_unwind(|| {
        let items = vec![Loud(2), Loud(7), Loud(4)];
        let _: (Pile, Pile) = items.into_iter().partition(|l| l.0 % 2 == 1);
    });
    println!("in extend: {}", in_extend.is_err());
}
