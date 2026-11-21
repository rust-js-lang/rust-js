//@ library-differs
// A generic `From` over an iterator, called through an `Into` bound's
// dictionary, is given the crate's iterator made the JS iterator its generic
// code steps, as a direct call gives it: the dictionary's entry wraps what
// it's given, where it gave the crate's iterator bare, a tuple struct's
// array, a fieldless enum's string or a unit struct's `undefined`, and
// `$next` threw (#2264).
struct Count(u8);

impl Iterator for Count {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        if self.0 >= 9 {
            return None;
        }
        self.0 += 1;
        Some(self.0)
    }
}

#[derive(Clone, Copy)]
enum Light {
    Red,
    Green,
}

impl Iterator for Light {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        *self = match self {
            Light::Red => Light::Green,
            Light::Green => Light::Red,
        };
        Some(*self as u8)
    }
}

struct Unit;

impl Iterator for Unit {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        Some(42)
    }
}

#[derive(Debug)]
struct One(u8);

impl<A: Iterator<Item = u8>> From<A> for One {
    fn from(mut a: A) -> Self {
        One(a.next().unwrap_or(0))
    }
}

fn converted<T, U: Into<T>>(u: U) -> T {
    u.into()
}

fn main() {
    println!("from {:?}", One::from(Count(3)));
    let one: One = Count(4).into();
    println!("into {one:?}");
    let one: One = converted(Count(5));
    println!("converted {one:?}");
    let one: One = converted(Light::Red);
    println!("converted light {one:?}");
    let one: One = converted(Unit);
    println!("converted unit {one:?}");
}
