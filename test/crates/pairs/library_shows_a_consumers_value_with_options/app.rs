//! A consumer whose own code shows nothing with options, giving its types
//! to its library's functions that do; and a type it gives none of them,
//! whose writer takes no options (#1331).
#[derive(Debug)]
struct Plain {
    n: u32,
}

/// Shown only as `Outer`'s field.
#[derive(Debug)]
struct Inner {
    n: u32,
}

#[derive(Debug)]
struct Outer {
    inner: Inner,
    tags: Vec<u16>,
}

#[derive(Debug)]
enum Labelled {
    One(u32),
}

#[derive(Debug)]
struct Untouched {
    n: u32,
}

pub fn main() {
    println!("{}", dep::hex(&Plain { n: 255 }));
    println!("{}", dep::pretty(&Plain { n: 2 }));
    let outer = Outer {
        inner: Inner { n: 26 },
        tags: vec![10, 11],
    };
    println!("{}", dep::pretty_hex(&outer));
    println!("{}", dep::labelled("one", &Labelled::One(3)));
    println!("{}", dep::hex(&vec![255u32, 16]));
    println!("{:?}", Untouched { n: 255 });
}
