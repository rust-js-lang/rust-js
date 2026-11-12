// A guard whose `Drop` does nothing is its fields (ADR 0608, 0705): a
// one-field one, `Held(v)`, read as `v`, and a named-field one, `Named {
// count, label }`, as `count` and `label`. Each field written stays one a
// `let` sets, or a guard's place: as a swap writes it, `[spare[0],
// pair.left] = [pair.left, spare[0]]`, where it was a `const` it assigned
// (#1570, #1593); as `(h[0] = next(i + 1))` sets it in a condition; and as
// `h[0]++` steps it.
use std::mem;

struct Held<T>(T);

impl<T> Drop for Held<T> {
    fn drop(&mut self) {}
}

struct Named {
    count: i32,
    label: &'static str,
}

impl Drop for Named {
    fn drop(&mut self) {}
}

struct Pair {
    left: Vec<i32>,
    right: i32,
}

fn next(n: i32) -> i32 {
    println!("next {n}");
    n * 3
}

fn swapped() {
    let mut pair = Held(Pair {
        left: vec![1, 2],
        right: 3,
    });
    let mut spare = Held(vec![7]);
    mem::swap(&mut spare.0, &mut pair.0.left);
    let mut named = Named {
        count: 4,
        label: "named",
    };
    let mut held = Held(5);
    mem::swap(&mut named.count, &mut pair.0.right);
    mem::swap(&mut held.0, &mut named.count);
    println!(
        "{:?} {:?} {} {} {} {}",
        spare.0, pair.0.left, pair.0.right, named.label, named.count, held.0
    );
}

fn set_in_conditions() {
    let mut h = Held(1.0);
    let mut g = Named {
        count: 2,
        label: "g",
    };
    for i in 0..4 {
        if next(i) > 4 || {
            h.0 = f64::from(next(i + 1));
            h.0 > 6.0
        } {
            println!("yes {}", h.0);
        }
        let more = next(i) > 4 || {
            g.count = next(i + 2);
            g.count > 9
        };
        println!("{more} {} {}", g.label, g.count);
    }
    println!("{} {}", h.0, g.count);
}

fn stepped() {
    let mut h = Held(1.5);
    let before = h.0;
    h.0 = before + 1.0;
    println!("{before} {}", h.0);
}

fn main() {
    swapped();
    set_in_conditions();
    stepped();
}
