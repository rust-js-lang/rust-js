// A `match` whose guarded or-pattern binds `x` in each alternative, its
// guard made again of the second, and whose next arm no value reaches,
// though its guard calls before it says so: an `if` chain, each guard made
// as often as Rust makes it, not MIR's labeled block. The binding's `let`
// before the block, past the arm's own declaration, is what the tree tests,
// and an arm whose guard calls is tested though its body never runs
// (#1620).
use std::cell::Cell;

fn seen(c: &Cell<u32>, n: u32) -> Option<u32> {
    let v = c.get();
    c.set(v + 2);
    println!("seen {n} {v}");
    if (v + n) % 4 == 1 { None } else { Some(v) }
}

fn odd(c: &Cell<u32>, n: u32) -> bool {
    let v = c.get();
    c.set(v + 1);
    println!("odd {n} {v}");
    (v * 7 + n) % 3 != 0
}

fn level(c: &Cell<u32>, a: u32, b: u32) -> u32 {
    let mut level = 7;
    match (a, b) {
        (x, _) | (x, 1) if seen(c, x).is_some() => level = x * 10,
        (0, 0) if odd(c, a + b) && a > 2 => return a + b + 1000,
        _ => level = 99,
    }
    level
}

// A way past one arm's guard that no value takes at another arm's test,
// `op === 0` once `op !== 0`, is no arm's: that guard is made in its own
// arm's test, and the chain reads as written.
fn chosen(c: &Cell<u32>, op: u32, l: u32) -> Result<u32, u32> {
    let a = op;
    Ok(match (op, l) {
        (0, 3) => 17,
        (1, _) | (_, 2) if seen(c, l).is_none() || a == 0 => 25,
        (2, _) | (_, 3) => return Err(7),
        (0, x) if seen(c, l).is_some() || x == 0 => x * 2 + 3,
        (3, x) => x * 2 + 4,
        (o, l) => return Err(o * 100 + l),
    })
}

fn main() {
    let c = Cell::new(0);
    for a in 0..4 {
        for b in 0..4 {
            println!("level({a}, {b}) = {}", level(&c, a, b));
            println!("chosen({a}, {b}) = {:?}", chosen(&c, a, b));
        }
    }
}
