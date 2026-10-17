// A value with a destructor made a `dyn` of std's that's a pair, `dyn Any`,
// `dyn Display`, `dyn Error`, is dropped through its dictionary's `$drop`,
// as a `dyn` of the crate's trait is: where Rust drops it, once.
use std::any::Any;
use std::fmt;
#[derive(Debug)]
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
impl fmt::Display for D {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "D{}", self.0)
    }
}
impl std::error::Error for D {}
fn fail(n: u32) -> Result<u32, Box<dyn std::error::Error>> {
    if n > 1 { Err(Box::new(D(n))) } else { Ok(n) }
}
fn main() {
    let a: Box<dyn Any> = Box::new(D(1));
    println!("{}", a.is::<D>());
    drop(a);
    let shown: Box<dyn fmt::Display> = Box::new(D(2));
    println!("{shown}");
    let items: Vec<Box<dyn Any>> = vec![Box::new(D(3)), Box::new(5u8)];
    println!("{}", items.len());
    match fail(4) {
        Err(e) => println!("err {e}"),
        Ok(n) => println!("{n}"),
    }
    println!("end");
}
