// A closure a function makes, given to a std consumer (#259): made once, not
// for each item, so what it changes of its own is kept from one to the
// next; and dropped, with what it took, where the consumer drops it, as
// std's `fold` does once it's done, a generic function's too.
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn counting() -> impl FnMut(i32, i32) -> i32 {
    let mut n = 0;
    move |a, x| {
        n += 1;
        a + x * n
    }
}

fn adder(tag: Loud) -> impl FnMut(i32, i32) -> i32 {
    move |a, x| {
        let _ = &tag;
        a * 10 + x
    }
}

fn joined<T, B, Acc>(mut f: impl FnMut(T) -> B, mut g: impl FnMut(Acc, B) -> Acc) -> impl FnMut(Acc, T) -> Acc {
    move |acc, x| g(acc, f(x))
}

fn fold_with<I: Iterator<Item = i32>, F: FnMut(i32) -> i32, Acc, G: FnMut(Acc, i32) -> Acc>(
    it: I,
    f: F,
    init: Acc,
    g: G,
) -> Acc {
    it.fold(init, joined(f, g))
}

fn main() {
    println!("{}", (1..5).fold(0, counting()));
    println!("{}", (1..4).fold(0, adder(Loud("adder's"))));
    let a = Loud("f");
    let b = Loud("g");
    let total = fold_with(
        1..4,
        move |x| {
            let _ = &a;
            x * 2
        },
        0,
        move |s, x| {
            let _ = &b;
            s * 10 + x
        },
    );
    println!("{total}");
}
