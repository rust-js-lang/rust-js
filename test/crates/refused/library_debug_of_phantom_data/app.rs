//! A consumer showing the library's `P<u8>`, which natively prints
//! `P { v: 1, m: PhantomData<u8> }`: refused, naming #1657.
use dep::P;

pub fn main() {
    println!("{:?}", P::<u8> { v: 1, m: std::marker::PhantomData });
}
