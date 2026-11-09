//! Each allocation apart, as Rust keeps it (#1053).
use dep::{boxed, boxed_generic, cloned, copied, double, shared};
use std::ptr::addr_eq;

pub fn main() {
    let (a, b) = (boxed(double), boxed(double));
    println!("boxed {} {}", addr_eq(&*a, &*b), a(1) + b(2));
    let k = 3;
    let add = move |x: u32| x + k;
    let (c, d) = (boxed_generic(add), boxed_generic(add));
    println!("generic {} {}", addr_eq(&*c, &*d), c(1) + d(2));
    let (e, f) = (shared(double), shared(double));
    println!("shared {} {}", addr_eq(&*e, &*f), e(1) + f(2));
    let (g, h) = copied(4);
    println!("copied {} {}", addr_eq(&*g, &*h), g(1) + h(2));
    let (i, j) = cloned("four".to_string());
    println!("cloned {} {}", addr_eq(&*i, &*j), i(1) + j(2));
}
