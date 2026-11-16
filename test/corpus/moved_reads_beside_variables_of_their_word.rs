// What tidy takes out, a temporary read once moved to its read, where
// another variable of the temporary's word is in scope: the program's own
// `value`, one shadowing it, and a closure's own (ADR 1150, step 3). Each
// asked of by its id, the temporary's read lands beside them, as the
// variable it is, and each drop and read is where Rust has it.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn make(n: u32) -> Loud {
    println!("make {n}");
    Loud(n)
}

fn pick(items: &[u32]) -> Option<u32> {
    items.iter().copied().max_by(|a, b| a.cmp(b))
}

fn main() {
    let value = make(1);
    let _ = make(4);
    println!("after {}", value.0);
    {
        let value = make(2);
        drop(make(5));
        println!("inner {}", value.0);
    }
    let value = make(3);
    let m = pick(&[3, 9, 4]);
    let read = |n: u32| {
        let value = n + 1;
        value * 2
    };
    println!("{} {:?} {}", value.0, m, read(value.0));
    let mut fails = 0;
    let count = |words: &[&str]| {
        let mut fails = 0;
        for w in words {
            if w.is_empty() {
                fails += 1;
            }
        }
        fails
    };
    fails += count(&["", "a", ""]);
    println!("fails {fails}");
    let mut chars = "xy".chars().enumerate();
    while let Some((i, c)) = chars.next() {
        println!("{i}{c}");
    }
    let value = chars.next();
    println!("{value:?}");
}
