// A guarded `|`'s bindings its arm reads, each alternative giving them
// other values: set in the guard's test, A1's form (#523, C1), so the
// alternatives are one arm of the chain. Two bindings, the guard reading
// one, `((x = b), (y = c), odd(x))`; a guard reading none,
// `((x = b), odd(a + b))`, made again of each alternative that matches;
// and a guard of what a call gives, `take(&c).is_some()`, which fails
// one way and passes another, each call made where Rust makes it.

use std::cell::Cell;

fn odd(n: u32) -> bool {
    println!("odd {n}");
    n % 2 == 1
}

fn both(rows: &[(u32, u32, u32)]) -> Vec<u32> {
    let mut seen = Vec::new();
    for &(a, b, c) in rows {
        let level = match (a, b, c) {
            (1, x, y) | (x, 1, y) if odd(x) => x * 10 + y,
            (2, 2, _) => 2,
            _ => 5,
        };
        seen.push(level);
    }
    seen
}

fn unread(pairs: &[(u32, u32)]) -> Vec<u32> {
    let mut seen = Vec::new();
    for &(a, b) in pairs {
        let level = match (a, b) {
            (1, x) | (x, 1) if odd(a + b) => x * 10,
            (2, 2) => 2,
            _ => 5,
        };
        seen.push(level);
    }
    seen
}

fn take(c: &Cell<u32>) -> Option<u32> {
    let v = c.get();
    c.set(v + 1);
    println!("take {v}");
    (v % 3 != 0).then_some(v)
}

fn taken(xs: &[u32]) -> Vec<u32> {
    let c = Cell::new(0);
    let mut seen = Vec::new();
    for &x in xs {
        let level = match x {
            0 => 0,
            1 | 2 if take(&c).is_some() => 1,
            1 | 2 => 2,
            _ => 3,
        };
        seen.push(level);
    }
    seen
}

fn main() {
    let rows = [(1, 1, 4), (1, 2, 5), (2, 1, 6), (3, 1, 7), (2, 2, 8), (1, 3, 9), (4, 4, 0)];
    println!("{:?}", both(&rows));
    let pairs = [(1, 1), (1, 2), (2, 1), (3, 1), (2, 2), (1, 4), (4, 1), (5, 5)];
    println!("{:?}", unread(&pairs));
    println!("{:?}", taken(&[0, 1, 2, 3, 1, 2, 1, 2, 0, 2]));
}
