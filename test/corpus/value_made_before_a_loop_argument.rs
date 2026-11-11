// A call's argument made before a later argument that's a `loop`, read
// where the loop breaks: made before the loop, where Rust makes it. Its
// `const` was dropped as the loop began, its name kept, so the `break`
// read a name nothing made, a `ReferenceError` (#1395).

#[inline(never)]
fn step(n: u32) -> u32 {
    n * 3 + 1
}

fn pair<A, B>(a: A, b: B) -> (A, B) {
    (a, b)
}

fn triple(a: i64, b: &str, c: u32) -> String {
    format!("{a} {b} {c}")
}

pub fn main() {
    // A value of arithmetic, the loop right after it.
    let mut k = 0;
    let r = pair(200 - 1, loop {
        k += 1;
        if k == 3 {
            break k;
        }
    });
    println!("{r:?}");

    // Of what's read: the loop changes `k` after it's read.
    let s = pair(k * 10, loop {
        k += 1;
        if k > 5 {
            break k;
        }
    });
    println!("{s:?} {k}");

    // Two made before it, of a call and of text.
    let mut n = 1;
    let t = triple(i64::from(step(2)) - 1, &"ab".repeat(2), loop {
        n = step(n);
        if n > 30 {
            break n;
        }
    });
    println!("{t}");

    // A `while` the loop is, its argument a negation.
    let mut i = 0u8;
    let w = pair(-(i64::from(step(4))), {
        while i < 4 {
            i += 1;
        }
        i
    });
    println!("{w:?}");
}
