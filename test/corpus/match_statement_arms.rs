// A `match` written as a statement, each arm setting a late `let`, stays
// that `match`'s `if`, as an `if` written so does (rd187): React's
// ViewTransitions writes `if (clones === null) { previousMeasurements =
// null; } else { previousMeasurements = clones.map(..); }`, and its port's
// `match clones.as_ref() { None => { .. = None; } Some(clones) => { .. } }`
// was one conditional (rd647). A `match` that's a value is one still,
// and so is an `if` that's a value in a `for`, which MIR writes in a `match`
// too, of no value, its desugaring's.

fn measured(clones: Option<&Vec<u32>>) -> usize {
    let previous: Option<Vec<u32>>;
    match clones {
        None => {
            previous = None;
        }
        Some(clones) => {
            previous = Some(clones.iter().map(|n| n * 2).collect());
        }
    }
    previous.map_or(0, |p| p.len())
}

fn armed(flag: Result<u32, u32>) -> u32 {
    let n;
    match flag {
        Ok(v) => n = v,
        Err(_) => n = 0,
    }
    n
}

fn inner(o: Option<u32>, flag: bool) -> u32 {
    let n;
    match o {
        Some(v) => {
            n = if flag { v } else { 1 };
        }
        None => {
            n = 2;
        }
    }
    n
}

fn valued(o: Option<u32>) -> u32 {
    let n = match o {
        Some(v) => v,
        None => 0,
    };
    n + 1
}

fn looped(xs: &[u32]) -> u32 {
    let mut total = 0;
    for &x in xs {
        let step = if x > 2 { x } else { 1 };
        total += step;
    }
    total
}

fn main() {
    println!("{} {}", measured(None), measured(Some(&vec![1, 2])));
    println!("{} {}", armed(Ok(4)), armed(Err(1)));
    println!("{} {} {}", inner(Some(5), true), inner(Some(5), false), inner(None, true));
    println!("{} {}", valued(Some(2)), valued(None));
    println!("{}", looped(&[1, 3, 5]));
}
