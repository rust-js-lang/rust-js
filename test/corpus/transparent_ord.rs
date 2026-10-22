// A derived order of a `#[repr(transparent)]` struct is its field's (ADR
// 0584): an id of bytes, as uuid's `Uuid` is, and a float's, `NaN` unordered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
struct Id([u8; 2]);

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[repr(transparent)]
struct Meters(f64);

fn main() {
    let (a, b) = (Id([1, 2]), Id([1, 3]));
    println!("{} {} {:?} {}", a < b, a == b, a.cmp(&b), a.max(b).0[1]);
    let mut ids = vec![b, a];
    ids.sort();
    println!("{ids:?}");
    println!("{} {:?}", Meters(1.0) < Meters(2.0), Meters(f64::NAN).partial_cmp(&Meters(1.0)));
}
