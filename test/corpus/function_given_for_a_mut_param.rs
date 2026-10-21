// A function given to generic code for its `F: FnMut(&mut T)`, of a `T`
// that's an object here: generic code calls it with its box of the object,
// which the function given takes out of the box, `(x0) => f(x0.value)`; of
// a number, the cell generic code has (ADR 0454).
struct Counter {
    n: i32,
}

fn apply<T, F: FnMut(&mut T)>(t: &mut T, mut f: F) {
    f(t);
    f(t);
}

fn bump(c: &mut Counter) {
    c.n += 10;
}

fn main() {
    let mut c = Counter { n: 1 };
    apply(&mut c, |x| x.n += 1);
    println!("{}", c.n);
    apply(&mut c, bump);
    println!("{}", c.n);
    let mut n = 5;
    apply(&mut n, |x| *x *= 2);
    println!("{n}");
}
