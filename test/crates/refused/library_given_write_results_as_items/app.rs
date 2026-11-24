//! Write results as the items of an iterator given to a library's generic
//! code that reads them as `Result`s of its type parameters: refused at the
//! call, as one crate refuses it (#2560), until each item is tagged where it
//! crosses (#2566).
use std::fmt;

pub fn main() {
    let v: Vec<fmt::Result> = vec![Ok(()), Err(fmt::Error), Ok(())];
    println!("{}", dep::oks(v.into_iter()));
}
