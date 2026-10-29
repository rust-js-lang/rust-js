// A value an `if` gives whose `then` is a conditional of its own keeps
// its branches where the program wrote them (ADR 0689, amended): React
// DOM's `blockingIndexSnapshot > 0 ? (more ? "Fonts and Images" :
// "Fonts") : "Images"`, not its test turned, the `else` first. A `match`'s
// arms, whose chain MIR may test the other way, keep theirs in the `else`.

fn reason(blocking: &[u32], snapshot: usize) -> &'static str {
    let blocked_reason = if snapshot > 0 {
        if blocking.len() > snapshot {
            "Waiting on Fonts and Images"
        } else {
            "Waiting on Fonts"
        }
    } else {
        "Waiting on Images"
    };
    blocked_reason
}

/// Of a signed count, and a value set late, `let`'s.
fn late(count: i32, more: bool) -> u32 {
    let level;
    if count > 0 {
        level = if more { 3 } else { 2 };
    } else {
        level = 1;
    }
    level * 10
}

/// Of an `||`, whose branches MIR turns, `!a && !b`'s.
fn ored(a: u32, b: bool, c: bool) -> u32 {
    let r = if a == 0 || b {
        if c { 1 } else { 2 }
    } else {
        3
    };
    r + 1
}

/// Both a conditional of their own: the `then` first, as written.
fn both(a: u8, b: bool, c: bool) -> char {
    let picked = if a == 0 {
        if b { 'w' } else { 'x' }
    } else if c {
        'y'
    } else {
        'z'
    };
    picked
}

/// A pair's part, each branch more than a value: a temporary's.
fn pushed(name: &str, indent: usize, out: &mut Vec<(usize, String)>) {
    out.push((
        indent,
        if name.is_empty() {
            if indent < 3 { "-".repeat(indent) } else { name.to_uppercase() }
        } else {
            name.repeat(2)
        },
    ));
}

/// A `match`'s arms, a chain in the `else`.
fn tier(variant: &str) -> u8 {
    let rank = match variant {
        "foundation" => 1,
        "intermediate" => 2,
        _ => 3,
    };
    rank + 1
}

fn main() {
    for snapshot in [0, 1, 3] {
        for len in [0, 2, 4] {
            let blocking = vec![7; len];
            println!("reason {snapshot} {len} {}", reason(&blocking, snapshot));
        }
    }
    for count in [-1, 0, 2] {
        for more in [true, false] {
            println!("late {count} {more} {}", late(count, more));
        }
    }
    for a in [0, 1] {
        for b in [true, false] {
            for c in [true, false] {
                println!("both {a} {b} {c} {}", both(a, b, c));
                println!("ored {a} {b} {c} {}", ored(a.into(), b, c));
            }
        }
    }
    let mut out = Vec::new();
    for name in ["", "ab"] {
        for indent in [1, 4] {
            pushed(name, indent, &mut out);
        }
    }
    println!("pushed {out:?}");
    for variant in ["foundation", "intermediate", "advanced"] {
        println!("tier {variant} {}", tier(variant));
    }
}
