//@ compile-fail: stepped through its
// Items JS holds as no object, a fieldless enum that is its own iterator,
// stepped by generic code through a bound of its items, `S::Item: Iterator`:
// they come as their caller holds them, bare, where the bound's `next`, of
// a `&mut` that's a cell, writes a place's `value`, so the call is refused
// (#2541), as items of a type parameter held so are (#2534), until each
// item is made a place where it crosses (#2535).

#[derive(Debug, Clone, Copy)]
enum Light {
    Red,
    Green,
}

impl Iterator for Light {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        let was = *self as u8;
        *self = match self {
            Light::Red => Light::Green,
            Light::Green => Light::Red,
        };
        Some(was)
    }
}

fn heads<S>(s: S) -> Vec<Option<<S::Item as Iterator>::Item>>
where
    S: Iterator,
    S::Item: Iterator,
{
    s.map(|mut i| i.next()).collect()
}

fn main() {
    println!("{:?}", heads([Light::Red, Light::Green].into_iter()));
}
