// rd72b: an `if` ending a loop's body whose condition mixes `&&`, `||` and
// `!`, around a loop of its own, is its one test as written, CSSPropertyOperations'
// validateShorthandPropertyCollisionInDev's `if (hasOwnProperty.call(next, key) &&
// (!prev || prev[key] !== next[key]))`, not `if (a) { if (b && c === d) { continue; } .. }`.
// A `continue` the program wrote stays.

fn changed(keys: &[i32], seen: bool, last: i32) -> Vec<i32> {
    let mut out = Vec::new();
    for &key in keys {
        if key > 0 && (!seen || last != key) {
            for part in [key, -key] {
                out.push(part);
            }
        }
    }
    out
}

fn written(keys: &[i32], seen: bool, last: i32) -> Vec<i32> {
    let mut out = Vec::new();
    for &key in keys {
        if key > 0 {
            if seen && last == key {
                continue;
            }
            for part in [key, -key] {
                out.push(part);
            }
        }
    }
    out
}

fn negated(keys: &[i32], seen: bool, last: i32) -> Vec<i32> {
    let mut out = Vec::new();
    for &key in keys {
        if !(key > 0 && (!seen || last != key)) {
            continue;
        }
        for part in [key, -key] {
            out.push(part);
        }
    }
    out
}

fn main() {
    for (seen, last) in [(false, 2), (true, 2), (true, 5)] {
        println!("{:?}", changed(&[1, 2, -3, 5], seen, last));
        println!("{:?}", written(&[1, 2, -3, 5], seen, last));
        println!("{:?}", negated(&[1, 2, -3, 5], seen, last));
    }
}
