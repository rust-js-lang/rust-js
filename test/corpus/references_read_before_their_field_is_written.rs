// A reference read out of a field, then the field written before the
// reference is used: Rust keeps what it read, the old one, where JS that
// read the field where it's used would see the new one. A `&str`, a `&dyn
// Fn` called and a fn pointer, each written while what's given is made.
struct Inner {
    n: u32,
    s: &'static str,
    f: &'static dyn Fn(u32) -> u32,
    g: fn(u32) -> u32,
}

fn double(n: u32) -> u32 {
    n * 2
}

fn triple(n: u32) -> u32 {
    n * 3
}

fn take(a: u32, b: u32) -> u32 {
    a * 10 + b
}

fn both(a: &str, b: u32) -> String {
    format!("{a}{b}")
}

fn number(inner: &mut Inner) -> u32 {
    take(inner.n, {
        inner.n = 5;
        1
    })
}

fn text(inner: &mut Inner) -> String {
    both(inner.s, {
        inner.s = "new";
        1
    })
}

fn called(inner: &mut Inner, other: &'static dyn Fn(u32) -> u32, n: u32) -> u32 {
    (inner.f)({
        inner.f = other;
        n
    })
}

fn pointer(inner: &mut Inner, other: fn(u32) -> u32, n: u32) -> u32 {
    (inner.g)({
        inner.g = other;
        n
    })
}

fn main() {
    let mut inner = Inner {
        n: 2,
        s: "old",
        f: &double,
        g: double,
    };
    println!("{}", number(&mut inner));
    println!("{}", text(&mut inner));
    println!("{}", called(&mut inner, &triple, 5));
    println!("{}", pointer(&mut inner, triple, 5));
    println!("{} {} {} {}", inner.n, inner.s, (inner.f)(5), (inner.g)(5));
}
