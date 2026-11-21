//! A consumer reading the library's public field of a fieldless enum's
//! iterator its generic code made, which it holds as it is where the
//! library's generic code holds it in a place of its own: refused (#2255).
#[derive(Debug)]
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
        Some(0)
    }
}

pub fn main() {
    let open = dep::open(Light::Red);
    println!("{:?}", open.inner);
}
