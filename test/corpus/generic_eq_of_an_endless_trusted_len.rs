// Generic code's `eq` of an endless `TrustedLen` iterator, `0..`, a
// `repeat(x)`, or a map, zip or chain of one: std's `SpecIterEq` reads its
// hint, `(usize::MAX, None)`, and gives `false` against a side of a known
// length at once, in either order, nothing stepped, its map's function
// never called; of two endless sides, compared item by item until two
// differ. Never collected first, which never ends (#2168).
use std::iter::repeat;

#[derive(Clone, PartialEq)]
struct L(u32);

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn equal<I, J>(a: I, b: J) -> bool
where
    I: IntoIterator,
    J: IntoIterator,
    I::Item: PartialEq<J::Item>,
{
    a.into_iter().eq(b)
}

fn main() {
    println!("{}", equal(0u32.., vec![0u32, 1]));
    println!("{}", equal(vec![0u32, 1], 0u32..));
    println!("{}", equal(0u32.., 1u32..));
    println!("{}", equal(0u64.., [0u64]));
    println!("{}", equal(0u8.., 0u8..3));
    println!("{}", equal(repeat(1u8), [1u8, 1]));
    println!("{}", equal([1u8], repeat(1u8)));
    println!("{}", equal(repeat(1u8), repeat(2u8)));
    println!("{}", equal((0u32..).zip(0u32..), (0u32..).zip(1u32..)));
    println!("{}", equal([1u8].into_iter().chain(repeat(1u8)), [1u8]));
    let doubled = (0u32..).map(|x| {
        println!("map {x}");
        x * 2
    });
    println!("{}", equal(doubled, [0u32, 2]));
    println!("{}", equal(repeat(L(1)), vec![L(2)]));
    println!("{}", equal(repeat(1u8).take(2), [1u8, 1]));
    println!("{}", equal(repeat(1u8).zip(0u32..2), [(1u8, 0u32), (1, 1)]));
}
