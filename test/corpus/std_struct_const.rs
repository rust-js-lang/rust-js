// A constant of std's `iter::Empty` is its JS value, `$iter([])`, an
// iterator over an array that has nothing in it (ADR 0128); each use a
// clone, as of any iterator over an array: what it has left, nothing.
const NOTHING: std::iter::Empty<u32> = std::iter::empty();

fn main() {
    for i in NOTHING {
        println!("{i}");
    }
    println!("{} {:?}", NOTHING.count(), NOTHING.max());
}
