// A type compared with another by its own `PartialEq<Other>`: the same
// impl through references, `Vec` and array `==`, and generic code bounded
// `A: PartialEq<B>`, as `a == b` asks it, of a struct and an enum.
struct Centimeters(i32);
struct Millimeters(i32);

impl PartialEq<Millimeters> for Centimeters {
    fn eq(&self, other: &Millimeters) -> bool {
        self.0 * 10 == other.0
    }
}

enum Unit {
    Cm(i32),
}

impl PartialEq<i32> for Unit {
    fn eq(&self, other: &i32) -> bool {
        match self {
            Unit::Cm(n) => n * 10 == *other,
        }
    }
}

fn same<A: PartialEq<B>, B>(a: &A, b: &B) -> bool {
    a == b
}

fn main() {
    let (a, b) = (Centimeters(2), Millimeters(20));
    println!("{} {} {}", a == b, &a == &b, same(&a, &b));
    println!("{}", vec![Centimeters(1)] == vec![Millimeters(10)]);
    println!("{}", [Centimeters(1), Centimeters(3)] == [Millimeters(10), Millimeters(31)]);
    let u = Unit::Cm(2);
    println!("{} {} {}", u == 20, &u == &20, [Unit::Cm(2)] == [20]);
}
