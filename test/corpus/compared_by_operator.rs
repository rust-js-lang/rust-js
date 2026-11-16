// Generic code's `PartialOrd` and `PartialEq` of a type parameter every
// caller gives a JS number, an integer of 32 bits or fewer or a float, are
// JS's own `<`, `<=`, `>`, `>=`, `===` and `!==`, no dictionary given: JS's
// operators order and compare numbers as Rust's do, `NaN` unordered and
// unequal to itself, `-0.0` equal to `0.0` (#1874). A `u32` instance and an
// `i32` one, each compared with its own kind; a generic caller that gives
// its own parameter on; and a function pointer to one.

fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn largest<T: PartialOrd + Copy>(items: &[T]) -> T {
    let mut most = items[0];
    for &item in items {
        if item > most {
            most = item;
        }
    }
    most
}

fn same<T: PartialEq>(a: T, b: T) -> bool {
    a == b
}

fn differ<T: PartialEq>(a: T, b: T) -> bool {
    a != b
}

fn ordered<T: PartialOrd>(a: T, b: T) -> [bool; 4] {
    [a < b, a <= b, a > b, a >= b]
}

// Its parameter given on to another's.
fn within<T: PartialOrd + Copy>(value: T, min: T, max: T) -> bool {
    !ordered(value, min)[0] && !ordered(value, max)[2] && same_order(value, max)
}

fn same_order<T: PartialOrd>(a: T, b: T) -> bool {
    a <= b
}

// Of an `Ord` bound, its `PartialOrd`'s operators.
fn at_most<T: Ord>(a: T, b: T) -> bool {
    a <= b
}

fn main() {
    println!("{} {} {}", clamp(5, 1, 3), clamp(-2, 1, 3), clamp(2, 1, 3));
    println!("{} {}", clamp(2.5, 0.0, 1.0), clamp(f64::NAN, 0.0, 1.0));
    println!("{} {}", clamp(7u32, 1, 5), clamp(u32::MAX, 0, 9));
    println!("{} {}", largest(&[3, -9, 12, 4]), largest(&[0.5f32, -1.5, 2.25]));
    println!("{}", largest(&[u32::MAX, 0, 7]));
    println!("{} {} {}", same(0.0, -0.0), same(f64::NAN, f64::NAN), same(3u8, 3));
    println!("{} {}", differ(f64::NAN, f64::NAN), differ(-1i16, -1));
    println!("{:?} {:?}", ordered(1, 2), ordered(f64::NAN, 1.0));
    println!("{:?} {:?}", ordered(u32::MAX, 1), ordered(-1i32, 1));
    println!("{} {}", within(4usize, 1, 9), within(-0.0, 0.0, 0.0));
    println!("{} {}", at_most(3u16, 3), at_most(9i8, -9));
    let pick: fn(i32, i32, i32) -> i32 = clamp;
    println!("{}", pick(10, 0, 4));
}
