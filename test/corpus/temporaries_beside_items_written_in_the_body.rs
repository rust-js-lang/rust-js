// A temporary rust-js names, beside a function or a `const` written in its
// body, which reads a function of the module's: the temporary takes no name
// of the module's (ADR 0569, amended). `match`'s value was named `note`, a
// call in one of its arms, and the `const note` hid the function `note` that
// `h1`, written beside it, calls, which threw `note is not a function` where
// Rust prints (nightly 43133510260's generated program 12461955). A
// closure's body, and a block's, the same; a body with no item keeps the
// names it gives.
fn id<T>(x: T) -> T {
    x
}

fn note<T>(tag: u32, x: T) -> T {
    println!("note {tag}");
    x
}

fn limit() -> i32 {
    40
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum E {
    A,
    B(i32),
}

fn pick(e: E) -> i32 {
    match e {
        E::A => id(-1i32),
        E::B(_) if id(false) => note(39, id(2i32)),
        E::B(_) => id(3i32),
    }
}

fn main() {
    let run = |k: i32| {
        fn h1(p1: i64) -> i64 {
            note(1, p1)
        }
        println!(
            "{:?}",
            (
                match E::A {
                    E::A => id(k),
                    E::B(_) if id(false) => note(39, id(2i32)),
                    E::B(_) => id(3i32),
                },
                None::<u8>.unwrap_or(id(127u8))
            )
        );
        h1(k as i64)
    };
    println!("{}", run(4));
    const TOP: i32 = 2;
    println!(
        "{:?}",
        (
            match E::B(1) {
                E::A => id(-1i32),
                E::B(_) if id(false) => limit(),
                E::B(_) => id(3i32),
            },
            None::<u8>.unwrap_or(id(127u8))
        )
    );
    {
        const CAP: i32 = TOP * 3;
        fn h2() -> i32 {
            limit() + CAP
        }
        println!("{} {}", h2(), pick(E::B(2)));
    }
}
