//@ library-differs
// A `dyn Fn` whose closure holds a `Box<dyn Error>`, and a `dyn FnOnce` whose
// closure holds a `Box<dyn Iterator>`, of a value with a destructor drop it as
// they drop: each `dyn`'s fact asked again till none changes (#1662).
use std::fmt;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl fmt::Debug for Loud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Loud")
    }
}

impl fmt::Display for Loud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "loud {}", self.0)
    }
}

impl std::error::Error for Loud {}

fn main() {
    let e: Box<dyn std::error::Error> = Box::new(Loud("e"));
    let f: Box<dyn Fn() -> String> = Box::new(move || e.to_string());
    println!("{}", f());
    drop(f);
    println!("after e");
    let it: Box<dyn Iterator<Item = u8>> = Box::new([Loud("i")].into_iter().map(|l| l.0.len() as u8));
    let g: Box<dyn FnOnce() -> u8> = Box::new(move || it.count() as u8);
    drop(g);
    println!("end");
}
