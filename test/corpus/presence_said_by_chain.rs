// A `match` of `Some`s of constants whose other arm leaves its loop, its
// value used after: each arm's test says the value is there, so where it
// isn't, the chain's `else` leaves, with no label for the `None` (bb11).
// A test that holds of `None` too, `n != 3 && n != 9`'s, is the presence
// test's too; a variable the arms give, declared again before the block,
// keeps the block.
const A: &str = "a";
const B: &str = "b";
const C: &str = "c";

fn levels(values: &[Option<&str>]) -> Vec<u32> {
    let mut seen = Vec::new();
    for value in values {
        let level;
        match *value {
            Some(A | B) => level = 1,
            Some(C) => level = 2,
            _ => {
                seen.push(0);
                break;
            }
        }
        seen.push(level);
    }
    seen
}

fn counted(values: &[Option<u32>]) -> Vec<u32> {
    let mut seen = Vec::new();
    for value in values {
        let level;
        match value {
            Some(1 | 2) => level = 10,
            Some(7) => level = 70,
            _ => {
                seen.push(0);
                break;
            }
        }
        seen.push(level);
    }
    seen
}

fn picked(values: &[Option<u32>]) -> Vec<u32> {
    let mut seen = Vec::new();
    for value in values {
        let level;
        match value {
            Some(1) => level = 10,
            Some(n) if *n != 3 && *n != 9 => level = *n,
            _ => {
                seen.push(0);
                break;
            }
        }
        seen.push(level);
    }
    seen
}

fn shadowed(values: &[Option<&str>]) -> Vec<u32> {
    let mut seen = Vec::new();
    for value in values {
        let level = seen.len() as u32;
        seen.push(level);
        let level;
        match *value {
            Some(A | B) => level = 1,
            Some(C) => level = 2,
            _ => {
                seen.push(0);
                break;
            }
        }
        seen.push(level);
    }
    seen
}

fn halves(values: &[Option<f64>]) -> Vec<f64> {
    let mut seen = Vec::new();
    for value in values {
        let level;
        match value {
            Some(1.0) => level = 10.0,
            Some(x) if x.is_nan() => level = 0.5,
            _ => {
                seen.push(0.0);
                break;
            }
        }
        seen.push(level);
    }
    seen
}

fn tallied(values: &[Option<u32>]) -> Vec<u32> {
    let mut seen = Vec::new();
    for value in values {
        let level;
        match value {
            Some(1 | 2) => level = 10,
            Some(7) => level = 70,
            _ => {
                seen.push(0);
                continue;
            }
        }
        seen.push(level);
    }
    seen
}

fn main() {
    println!("{:?}", levels(&[Some("a"), Some("c"), Some("x"), Some("b")]));
    println!("{:?}", levels(&[Some("b"), None, Some("a")]));
    println!("{:?}", levels(&[Some("c"), Some("a")]));
    println!("{:?}", counted(&[Some(1), Some(7), None, Some(2)]));
    println!("{:?}", counted(&[Some(2), Some(3)]));
    println!("{:?}", picked(&[Some(1), Some(5), None, Some(2)]));
    println!("{:?}", picked(&[Some(4), Some(3), Some(1)]));
    println!("{:?}", halves(&[Some(1.0), Some(f64::NAN), Some(2.0), Some(1.0)]));
    println!("{:?}", halves(&[Some(1.0), None, Some(1.0)]));
    println!("{:?}", tallied(&[Some(1), None, Some(7), Some(3), Some(2)]));
    println!("{:?}", shadowed(&[Some("a"), Some("c"), None, Some("b")]));
}
