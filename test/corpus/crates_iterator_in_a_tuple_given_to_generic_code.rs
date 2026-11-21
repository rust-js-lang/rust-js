//@ library-differs
// The crate's iterator held in a tuple, given where generic code takes the
// tuple and steps that part as a JS iterator, is given that part made one,
// by every route: a direct call, `into()`, a generic function and an `Into`
// bound's dictionary; a tuple written there item by item, another remade
// part by part by `$inPlace`. It was given bare, a tuple struct's array
// stepped as its items (#2268).
struct Count(u8);

impl Iterator for Count {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
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

#[derive(Debug)]
struct Two(u8);

impl<A: IntoIterator<Item = u8>> From<(A, u8)> for Two {
    fn from((a, n): (A, u8)) -> Self {
        Two(a.into_iter().next().unwrap_or(0) + n)
    }
}

fn pair<A: IntoIterator<Item = u8>>((a, n): (A, u8)) -> u8 {
    a.into_iter().next().unwrap_or(0) + n
}

fn nested<A: Iterator<Item = u8>, B: Iterator<Item = u8>>(((mut a, n), mut b): ((A, u8), B)) -> u8 {
    a.next().unwrap_or(0) + n + b.next().unwrap_or(0)
}

fn converted<T, U: Into<T>>(u: U) -> T {
    u.into()
}

fn main() {
    println!("direct {:?}", Two::from((Count(1), 10)));
    let two: Two = (Count(1), 10).into();
    println!("into {two:?}");
    println!("generic fn {}", pair((Count(1), 10)));
    let two: Two = converted((Count(1), 10));
    println!("converted {two:?}");
    let held = (Count(2), 20);
    println!("held {}", pair(held));
    let two: Two = converted((Light::Red, 30));
    println!("light {two:?}");
    println!("nested {}", nested(((Count(3), 40), Light::Green)));
}
