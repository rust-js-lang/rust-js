// A `match` of an `Option`'s constants, `Some(7) => ..`, whose `None` goes
// where its other values do, is a test of the value itself, `sub === 7`,
// which no `None` passes: one `switch (sub)`, no `sub != null` before it,
// and no labeled block where its arms leave a block early, as React's
// createFiberFromTypeAndProps' `break getTag` (ADR 0629, amended). Of a
// `char`'s, a reference's, and one whose `None` goes elsewhere, too.

fn tag_of(sub: Option<u8>) -> &'static str {
    let mut tag = "function";
    'get_tag: {
        match sub {
            Some(7) => {
                tag = "context";
                break 'get_tag;
            }
            Some(8) => {
                tag = "memo";
                break 'get_tag;
            }
            _ => {}
        }
        tag = "throw";
    }
    tag
}

fn letter(c: &Option<char>) -> u32 {
    match *c {
        Some('a') => 1,
        Some('b') | Some('c') => 2,
        _ => 0,
    }
}

fn apart(n: Option<i32>) -> &'static str {
    match n {
        Some(1) => "one",
        Some(2) => "two",
        None => "none",
        Some(_) => "other",
    }
}

// Of a `bool`, which JS tests by its truth, `!b`, which `None` passes: its
// test that it's there first.
fn flag(b: Option<bool>) -> u8 {
    match b {
        Some(false) => 1,
        _ => 0,
    }
}

const ENABLE_A: bool = false;
const ENABLE_B: bool = true;
const ENABLE_C: bool = false;

// A case the program says falls through, `// falls through`, has no
// `// Fallthrough` beside it (ADR 0905, amended).
fn fallen(kind: u8) -> &'static str {
    match kind {
        0 => return "zero",
        1 if ENABLE_A => return "a",
        // falls through
        1 | 2 if ENABLE_B => return "b",
        // falls through
        1 | 2 | 3 if ENABLE_C => return "c",
        _ => {}
    }
    "none"
}

fn main() {
    for b in [None, Some(false), Some(true)] {
        println!("{:?} {}", b, flag(b));
    }
    for kind in 0..5 {
        println!("{kind} {}", fallen(kind));
    }
    for sub in [None, Some(7), Some(8), Some(9)] {
        println!("{:?} {}", sub, tag_of(sub));
    }
    for c in [None, Some('a'), Some('c'), Some('z')] {
        println!("{:?} {}", c, letter(&c));
    }
    for n in [None, Some(1), Some(2), Some(3)] {
        println!("{:?} {}", n, apart(n));
    }
}
