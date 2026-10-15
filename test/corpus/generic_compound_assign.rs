// A `T: AddAssign` and its kin are given a dictionary (ADR 0396): std's of
// numbers and of text assigns the box's `value`, as `a += b` of a known
// number does; the crate's own is its impl's. Each side runs in Rust's
// order: of a number, the right first, as MIR has it; of another, the left.
use std::ops::{AddAssign, MulAssign, ShlAssign, SubAssign};

#[derive(Debug, Default, Clone, Copy)]
struct Money(i64);

impl AddAssign for Money {
    fn add_assign(&mut self, other: Money) {
        println!("add {}", other.0);
        self.0 += other.0;
    }
}

fn total<T: AddAssign + Default + Copy>(items: &[T]) -> T {
    let mut sum = T::default();
    for &item in items {
        sum += item;
    }
    sum
}

fn scaled<T: MulAssign + SubAssign + Copy>(mut x: T, by: T, less: T) -> T {
    x *= by;
    x -= less;
    x
}

fn shifted<T: ShlAssign<u32>>(mut x: T) -> T {
    x <<= 3;
    x
}

fn joined<T: for<'a> AddAssign<&'a str>>(mut text: T) -> T {
    text += ", world";
    text
}

struct Wrapper<T>(T);

fn ordered<T: AddAssign + Default>(log: &mut Vec<&'static str>) {
    let mut held = Wrapper(T::default());
    {
        log.push("left");
        &mut held
    }
    .0 += {
        log.push("right");
        T::default()
    };
}

fn main() {
    println!("{} {:?}", total(&[1u8, 2, 250]), total(&[Money(5), Money(7)]));
    println!("{} {}", scaled(3i32, 4, 5), scaled(1.5f64, 2.0, 0.25));
    println!("{} {}", shifted(1u64), joined(String::from("hello")));
    let mut log = vec![];
    ordered::<i32>(&mut log);
    ordered::<Money>(&mut log);
    println!("{log:?}");
    // The crate's own, of a field, where its type is known: its impl's.
    let mut purse = Wrapper(Money(1));
    {
        log.push("purse");
        &mut purse
    }
    .0 += Money(2);
    println!("{:?} {log:?}", purse.0);
}
