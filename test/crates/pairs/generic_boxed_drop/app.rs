//! Given a value with a destructor: dropped where Rust drops it, with the
//! `Vec` that holds it, once it's printed `held`.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn main() {
    println!("{}", dep::hold(Loud(1)));
    println!("{}", dep::hold(vec![Loud(2), Loud(3)]));
}
