// A zip of what holds nothing to drop, a slice's `iter_mut()`, a `chars()`
// or a range, and a `Vec` whose items
// have a destructor, is closed as it's dropped, which drops what's left of
// the `Vec`'s, as Rust's drops them, at its end and where a loop breaks
// out of it (#406).

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn louds(items: &mut [Loud], by: Vec<Loud>) {
    for (item, other) in items.iter_mut().zip(by) {
        item.0 += other.0;
    }
}

fn until(by: Vec<Loud>) {
    for (n, l) in (0..10).zip(by) {
        if l.0 > 20 {
            println!("stop at {n}");
            break;
        }
        println!("{n} {}", l.0);
    }
}

fn main() {
    let mut items = vec![Loud(1), Loud(2)];
    louds(&mut items, vec![Loud(10), Loud(20), Loud(30)]);
    println!("{} {}", items[0].0, items[1].0);
    for (c, l) in "ab".chars().zip(vec![Loud(41), Loud(42), Loud(43)]) {
        println!("{c} {}", l.0);
    }
    until(vec![Loud(11), Loud(22), Loud(33)]);
}
