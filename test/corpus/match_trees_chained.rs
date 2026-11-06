// `match`es MIR tests as trees through a labeled block, each the chain of
// its arms, no label the Rust doesn't write (#523's C1, #771), of shapes
// test/match-trees.ts generates, unreachable patterns too: a guard's
// binding read as its value in each test of the guard, the nested `if`s
// MIR makes of its `&&`; a binding the guard doesn't read beside one it
// does; the bindings of two alternatives, each given again for the other's
// guard; a binding MIR declares between the tree's tests, declared before
// the chain, and one of a name declared outside it too, which stays; an arm's own
// `match` after an arm that leaves the outer one; a guard's alternative a
// test before decides, `(_, _) | (0, 2) if .. || a == 0`; a guard's read
// after its call, kept after it; a guard of what the `match` keeps, read
// after another guard's call; and a binding nothing reads. Each guard's
// calls are made as often, and in the order, as natively. An arm's own
// `match` whose value is given between arms that return, no chain then,
// each `if` that leaves the block nesting what follows it (the user's
// decision, #771); a guard's binding MIR declares, `let x = a;`, read as
// its value, of the block and of an arm's own tree past its first leaving;
// and a presence test read once, past the arm of its negation, but
// another value's read where it was (#791); an arm's statement copied
// into each alternative that binds its value apart, each its own binding
// (the user's decision A, #771); and a guard's test of constants alone,
// or of a parameter the function never replaces, read after another
// guard's call, which no call changes (#771).
use std::cell::Cell;

fn g(c: &Cell<u32>, n: u32, k: u32) -> bool {
    let v = c.get();
    c.set(v + 1);
    println!("g{k} {n} {v}");
    (v * 7 + n) % 3 != 0
}

fn h(c: &Cell<u32>, n: u32) -> Option<u32> {
    let v = c.get();
    c.set(v + 2);
    println!("h {n} {v}");
    if (v + n) % 4 == 1 { None } else { Some(v) }
}

fn step(c: &Cell<u32>, v: u32) -> Result<u32, u32> {
    let n = c.get();
    c.set(n + 1);
    if (n + v) % 5 == 4 { Err(v) } else { Ok(v % 4) }
}

fn read_twice(c: &Cell<u32>, e: Option<(u32, u32)>) -> Result<u32, u32> {
    Ok(match e {
        None => 5,
        Some((0, 0)) => 6,
        Some((op, v)) => {
            let l = step(c, v)?;
            match (op, l) {
                (3, x) if x % 2 == 0 && g(c, x, 0) => x * 2,
                (0, _) | (_, 3) => return Err(17),
                (2, x) => x * 2 + 3,
                (o, l) => return Err(o * 100 + l),
            }
        }
    })
}

fn one_read(s: &[u32]) -> u32 {
    match s {
        &[x, .., y] if y * 2 > 1 => x * 2,
        &[0] | &[.., 1] => 0,
        _ => 99,
    }
}

fn given_again(c: &Cell<u32>, s: &[u32]) -> u32 {
    let mut level = 0;
    for _ in 0..2 {
        level += 1;
        match s {
            &[x, y, ..] | &[x, y, ..] if g(c, x + y, 0) => break,
            &[.., 1] | &[] => level += 5,
            _ => level += 1,
        }
        level += 10;
    }
    level
}

fn made_again(c: &Cell<u32>, s: &[u32]) -> u32 {
    let mut level = 7;
    match s {
        &[1, x, _] if g(c, x, 0) => level = x + 1,
        &[x, .., _] | &[x, _, ..] if g(c, x + 1, 1) => level = x * 2,
        _ => level = 99,
    }
    level
}

fn declared_between(c: &Cell<u32>, a: u32, b: u32) -> u32 {
    match (a, b) {
        (_, 2) | (_, 0) | (3, 0) => a + b,
        (x, 3) | (3, x) | (x, 0) if h(c, x).is_none() && x > 2 => x + a,
        _ => 99,
    }
}

fn declared_outside(c: &Cell<u32>, e: Option<(u32, u32)>) -> Result<u32, u32> {
    Ok(match e {
        None => 5,
        Some((0, 0)) => 6,
        Some((op, v)) => {
            let l = step(c, v)?;
            let (a, b) = (op, l);
            match (op, l) {
                (1, x) => x * 2 + 1,
                (2, x) | (x, 2) if h(c, x).is_none() || x == 0 => return Err(x * 2),
                (1, 3) if h(c, b).is_none() && a > 2 => 8,
                (3, x) | (x, 2) => x * 2 + 2,
                (3, x) if !g(c, x, 2) => x * 2 + 1,
                (o, l) => return Err(o * 100 + l),
            }
        }
    })
}

fn declared_twice(c: &Cell<u32>, a: u32, b: u32) -> u32 {
    let mut out = Vec::new();
    for t in 0..3u32 {
        let (a, b) = (a + t % 2, b);
        match (a, b) {
            (x, 2) | (2, x) if x % 2 == 0 && !g(c, x, 0) => out.push(x),
            (1, 0) | (0, 0) if h(c, a).is_some() => continue,
            (2, 3) if g(c, a, 2) => continue,
            (1, x) | (0, x) | (_, x) => continue,
            _ => break,
        }
        out.push(a * 100 + t);
    }
    out.iter().sum::<u32>() + out.len() as u32
}

fn inner_after_an_arm(c: &Cell<u32>, e: Option<(u32, u32)>) -> Result<u32, u32> {
    Ok(match e {
        None => 5,
        Some((0, 0)) => 6,
        Some((op, v)) => {
            let l = step(c, v)?;
            match (op, l) {
                (0, x) if x % 2 == 0 && g(c, x, 0) => x * 2 + 4,
                (2, x) => x * 2 + 3,
                (1, _) | (_, 3) => return Err(0),
                (3, x) | (x, 3) => return Err(x * 2 + 3),
                (o, l) => return Err(o * 100 + l),
            }
        }
    })
}

fn decided_alternative(c: &Cell<u32>, a: u32, b: u32) -> u32 {
    let mut level = 7;
    match (a, b) {
        (_, _) | (0, 2) if h(c, a * 2).is_none() || a == 0 => level = b * 3,
        (1, x) if !g(c, x, 1) || x == 0 => return x + 1000,
        (1, x) | (x, 3) if h(c, x).is_none() => level = x + a,
        (_, 2) | (_, _) => level = 0,
        _ => level = 99,
    }
    level
}

fn read_after_call(c: &Cell<u32>, a: u32, b: u32) -> u32 {
    let mut level = 7;
    match (a, b) {
        (2, x) | (x, 0) if g(c, x, 0) => level = x,
        (3, x) if h(c, x).is_none() || x == 2 => return x * 10 + 1000,
        (0, x) | (x, 0) if h(c, a * 2).is_none() && x > 0 => return x + a + 1000,
        (3, 3) => level = a + b,
        _ => level = 99,
    }
    level
}

fn kept_read(c: &Cell<u32>, a: u32, b: u32) -> u32 {
    let mut level = 0;
    for _ in 0..2 {
        level += 1;
        match (a, b) {
            (x, 1..=2) | (x, 1) if g(c, x + 3, 0) => {}
            (3, _) | (_, 3) if !g(c, a * 2, 1) && g(c, 4, 1) => continue,
            (0, x) | (3, x) if a % 2 == 0 => break,
            _ => level += 1,
        }
        level += 10;
    }
    level
}

fn ways_none_takes(c: &Cell<u32>, a: u32, b: u32) -> u32 {
    let mut out = Vec::new();
    for t in 0..3u32 {
        let (a, b) = (a + t % 2, b);
        match (a, b) {
            (0, x) | (x, 3) if x % 2 == 0 && h(c, x).is_none() => continue,
            (1, 2) => break,
            (2, _) | (3, 0) | (3, _) => {}
            (x, 2) | (x, 1) if g(c, a * 2, 1) && x > 1 => {}
            _ => continue,
        }
        out.push(a * 100 + t);
    }
    out.iter().sum::<u32>() + out.len() as u32
}

fn given_before_the_tree(c: &Cell<u32>, a: u32, b: u32) -> u32 {
    match (a, b) {
        (0, x) => x * 10,
        (1, 1) if g(c, a + b, 0) || a == 2 => 0,
        (x, _) | (x, 0) if h(c, x).is_some() || x == 0 => x * 10,
        _ => 99,
    }
}

fn declared_first_and_between(c: &Cell<u32>, a: u32, b: u32) -> u32 {
    match (a, b) {
        (1, 1) | (0, 3) | (_, 1) if g(c, a + b, 0) => b * 3,
        (x, 3) | (0, x) if h(c, x).is_some() && x > 2 => x * 10,
        (x, 1) | (0, x) | (2, x) if !g(c, x, 2) && x > 1 => x,
        (0, x) if !g(c, x, 3) || x == 0 => x * 10,
        _ => 99,
    }
}

fn value_between_returns(c: &Cell<u32>, e: Option<(u32, u32)>) -> Result<u32, u32> {
    Ok(match e {
        None => 5,
        Some((0, 0)) => 6,
        Some((op, v)) => {
            let l = step(c, v)?;
            match (op, l) {
                (3, x) => x * 2 + 2,
                (1, 0) => return Err(13),
                (0, x) => return Err(x * 2 + 1),
                (o, l) => return Err(o * 100 + l),
            }
        }
    })
}

fn either_end(s: &[u32]) -> u32 {
    let mut level = 7;
    match s {
        &[x, ..] | &[.., x] if x * 2 % 4 == 0 => level = 0,
        &[_, 3, ..] | &[] if level > 3 => level = 4,
        _ => level = 99,
    }
    level
}

fn given_twice(c: &Cell<u32>, a: u32, b: u32) -> u32 {
    match (a, b) {
        (_, 1) if 2 + a > 1 => 2 * 2,
        (x, 2) | (x, 2) | (2, x) if x > 3 || g(c, b, 1) => 2,
        (1, _) | (_, 0) | (1, _) if !g(c, b * 2, 2) => 0,
        _ => 99,
    }
}

fn presence_of_another(o: Option<u32>, p: Option<(u32, u32)>) -> Result<u32, u32> {
    Ok(match (o, p) {
        (None, _) => 5,
        (_, Some((0, 0))) => 6,
        (Some(a), Some((x, y))) if a > 1 => x + y,
        (Some(a), _) => return Err(a),
    })
}

fn bound_in_each(c: &Cell<u32>, e: Option<(u32, u32)>) -> Result<u32, u32> {
    Ok(match e {
        None => 5,
        Some((0, 0)) => 6,
        Some((op, v)) => {
            let l = step(c, v)?;
            match (op, l) {
                (0, x) | (x, 3) => x * 2 + 2,
                (2, x) => x * 2 + 3,
                (o, l) => return Err(o * 100 + l),
            }
        }
    })
}

const SLICES: &[&[u32]] = &[
    &[],
    &[1],
    &[2],
    &[1, 1],
    &[2, 2],
    &[1, 2],
    &[2, 1],
    &[3, 1, 2],
    &[1, 3, 1],
    &[0, 1, 2, 3],
];

fn param_after_call(c: &Cell<u32>, o: Option<u32>) -> u32 {
    let mut level = 0;
    for _ in 0..2 {
        level += 1;
        match o {
            Some(_x) | Some(_x) if g(c, 4, 0) || 0 % 2 == 0 => break,
            Some(x) if 2 > 1 && x % 2 == 0 => continue,
            _ => {
                level += 1;
            }
        }
        level += 10;
    }
    level
}

fn constant_after_call(c: &Cell<u32>, o: Option<u32>) -> u32 {
    match o {
        Some(x) | Some(x) if x + 3 % 2 == 0 || 2 % 2 == 0 => x,
        Some(_x) if g(c, 3, 1) && 3 % 2 == 0 => 1,
        None | Some(1..=2) if g(c, 2 * 2, 2) => 1 + 1,
        None => 0,
        _ => 99,
    }
}

fn main() {
    let c = &Cell::new(0);
    for op in 0..4u32 {
        for v in 0..4u32 {
            println!("read_twice {:?}", read_twice(c, Some((op, v))));
            println!("declared_outside {:?}", declared_outside(c, Some((op, v))));
            println!("inner_after_an_arm {:?}", inner_after_an_arm(c, Some((op, v))));
            println!("value_between_returns {:?}", value_between_returns(c, Some((op, v))));
            println!("bound_in_each {:?}", bound_in_each(c, Some((op, v))));
        }
    }
    println!("read_twice {:?}", read_twice(c, None));
    println!("declared_outside {:?}", declared_outside(c, None));
    println!("inner_after_an_arm {:?}", inner_after_an_arm(c, None));
    println!("value_between_returns {:?}", value_between_returns(c, None));
    println!("bound_in_each {:?}", bound_in_each(c, None));
    for o in [None, Some(0), Some(1), Some(2), Some(5)] {
        println!("constant_after_call {}", constant_after_call(c, o));
        println!("param_after_call {}", param_after_call(c, o));
    }
    for o in [None, Some(0), Some(2)] {
        for p in [None, Some((0, 0)), Some((1, 2))] {
            println!("presence_of_another {:?}", presence_of_another(o, p));
        }
    }
    for s in SLICES {
        println!("one_read {}", one_read(s));
        println!("given_again {}", given_again(c, s));
        println!("made_again {}", made_again(c, s));
        println!("either_end {}", either_end(s));
    }
    for a in 0..4u32 {
        for b in 0..4u32 {
            println!("declared_between {}", declared_between(c, a, b));
            println!("declared_twice {}", declared_twice(c, a, b));
            println!("decided_alternative {}", decided_alternative(c, a, b));
            println!("read_after_call {}", read_after_call(c, a, b));
            println!("kept_read {}", kept_read(c, a, b));
            println!("ways_none_takes {}", ways_none_takes(c, a, b));
            println!("given_before_the_tree {}", given_before_the_tree(c, a, b));
            println!("declared_first_and_between {}", declared_first_and_between(c, a, b));
            println!("given_twice {}", given_twice(c, a, b));
        }
    }
}
