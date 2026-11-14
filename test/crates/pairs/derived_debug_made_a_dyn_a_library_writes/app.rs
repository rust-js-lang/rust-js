//! A program's types deriving `Debug`, each made a `dyn Debug` and given to
//! a library that writes it into its writer a piece at a time: each piece
//! is a call of the library's writer, as Rust's is (#1774).
use dep::{Log, describe};

#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
}

pub fn main() {
    let view = Viewport { x: 1.5, y: -2.0 };
    let bounds = Some(view);
    let mut log = Log(Vec::new());
    describe(&mut log, &view).unwrap();
    describe(&mut log, &bounds).unwrap();
    println!("{:?}", log.0);
}
