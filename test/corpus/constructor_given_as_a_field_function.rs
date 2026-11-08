// A field of a function type, filled by a tuple struct's constructor,
// `make: Loud`: its parameters, declared, named by none of what fills it,
// as a constructor names none, where asking rustc its names crashed
// rust-js (#952).
pub struct Loud(pub u8);

pub struct Make {
    pub make: fn(u8) -> Loud,
}

pub fn made(maker: &Make, n: u8) -> u8 {
    (maker.make)(n).0
}

fn main() {
    let maker = Make { make: Loud };
    println!("{}", made(&maker, 7));
}
