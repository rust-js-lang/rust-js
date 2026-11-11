//! A consumer whose own code shows nothing with options, making its types
//! and std's `dyn Debug`s for its library's functions that do; and a type
//! it makes none of, whose writer takes no options (#1335).
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
struct Untouched {
    n: u32,
}

pub fn main() {
    println!("{}", dep::pretty(&Plain { n: 2 }));
    println!("{}", dep::hex(&Plain { n: 255 }));
    let outer = Outer {
        inner: Inner { n: 26 },
        tags: vec![10, 11],
    };
    println!("{}", dep::pretty_hex(&outer));
    println!("{}", dep::hex(&vec![255u32, 16]));
    println!("{}", dep::pretty(&Some((1u8, "a"))));
    println!("{}", dep::all_pretty(vec![Box::new(Plain { n: 3 }), Box::new(17u8)]));
    println!("{:?}", Untouched { n: 255 });
}
