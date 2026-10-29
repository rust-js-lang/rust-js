// A `match` of an `Option`'s constants whose `_` does something, `Some("a"
// | "b") => .., Some("c") => continue, _ => hit = false`: `switch (x) {
// .. default: hit = false; }`, `None` the `default`'s as each value no case
// takes is, not `bb22: { if (x != null) { switch (x) { .. break bb22; } }
// hit = false; }` (#221).

fn kinds(names: &[Option<&str>]) -> Vec<u32> {
    let mut seen = Vec::new();
    for name in names {
        let mut level = 0;
        match name.as_deref() {
            Some("a" | "b") => level = 1,
            Some("c") => continue,
            _ => {
                seen.push(9);
                level = 3;
            }
        }
        seen.push(level);
    }
    seen
}

fn lanes(values: &[Option<u32>], limit: u32) -> Vec<u32> {
    let mut seen = Vec::new();
    for value in values {
        let mut level = limit;
        let below = |n: u32| n < level;
        match value {
            Some(1 | 2) => level = 1,
            Some(3) if below(3) => level = 3,
            Some(4) => return seen,
            _ => level = 5,
        }
        seen.push(level);
    }
    seen
}

// A loop's `break` in the rest stays the loop's: no `default` of it.
fn stopped(values: &[Option<u32>]) -> Vec<u32> {
    let mut seen = Vec::new();
    for value in values {
        let mut level = 0;
        match value {
            Some(1 | 2) => level = 1,
            Some(3) => continue,
            _ => {
                seen.push(0);
                if seen.len() > 3 {
                    break;
                }
            }
        }
        seen.push(level);
    }
    seen
}

fn main() {
    let names = [Some("a"), None, Some("c"), Some("x"), Some("b"), None];
    println!("{:?}", kinds(&names));
    let values = [Some(1), Some(3), None, Some(9), Some(2), Some(4), Some(1)];
    println!("{:?} {:?}", lanes(&values, 9), lanes(&values, 2));
    println!("{:?}", stopped(&[None, Some(1), Some(7), Some(3), None, Some(2), None, None]));
}
