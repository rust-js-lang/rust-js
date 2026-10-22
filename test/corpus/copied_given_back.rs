// A `copied()` given back as an `impl Iterator`, as regex-syntax's ASCII
// classes are: the iterator it copies, each item a copy where it's changed;
// of numbers, which nothing changes in place, the iterator itself.

fn class(lower: bool) -> impl Iterator<Item = (u8, u8)> {
    let slice: &'static [(u8, u8)] = match lower {
        true => &[(b'a', b'z')],
        false => &[(b'0', b'9'), (b'A', b'Z')],
    };
    slice.iter().copied()
}

fn bytes() -> impl Iterator<Item = u8> {
    b"ab".iter().copied()
}

fn main() {
    let all: Vec<(u8, u8)> = class(false).collect();
    let widths: Vec<u8> = class(true).map(|(a, b)| b - a).collect();
    let mut first = class(false).next().unwrap();
    first.0 = 1;
    println!("{all:?} {widths:?} {first:?} {:?}", class(false).next());
    println!("{:?}", bytes().collect::<Vec<_>>());
}
