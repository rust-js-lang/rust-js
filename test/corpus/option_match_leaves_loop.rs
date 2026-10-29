// A `match` of an `Option`'s constants whose arm leaves the loop, `Some(3)
// => break`, which no `switch` holds without a label the Rust lacks: an
// `if` chain, each constant's test saying it's no `None`, `code === 3`, not
// `code != null && code === 3` in each arm and MIR's labeled block (#412).

fn first(items: &[Option<u32>]) -> Vec<bool> {
    let mut hit = false;
    let mut seen = Vec::new();
    for &item in items {
        match item {
            Some(1 | 2) => hit = true,
            Some(3) => break,
            _ => hit = false,
        }
        seen.push(hit);
    }
    seen.push(hit);
    seen
}

fn named(items: &[Option<&str>]) -> Vec<u32> {
    let mut seen = Vec::new();
    for &item in items {
        let mut level = 0;
        match item {
            Some("a" | "b") => level = 1,
            Some("c") => continue,
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
    println!("{:?}", first(&[Some(1), None, Some(4), Some(2), Some(3), Some(1)]));
    println!("{:?}", first(&[None, Some(2)]));
    println!(
        "{:?}",
        named(&[Some("a"), Some("c"), None, Some("b"), Some("x"), Some("x"), Some("a")])
    );
}
