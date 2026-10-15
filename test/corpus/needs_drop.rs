// `mem::needs_drop::<T>()` is rustc's answer for the type, a literal; of a
// type parameter, its caller's, given as its other facts are (ADR 0145).
use std::mem::needs_drop;
struct Plain(u32);
struct Noisy;
impl Drop for Noisy { fn drop(&mut self) {} }
fn generic<T>() -> bool { needs_drop::<T>() }
fn main() {
    println!("{} {} {} {}", needs_drop::<u32>(), needs_drop::<String>(), needs_drop::<Plain>(), needs_drop::<Noisy>());
    println!("{} {} {}", generic::<Noisy>(), generic::<i8>(), generic::<Vec<u8>>());
}
