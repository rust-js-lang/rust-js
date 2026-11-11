//! A consumer giving the library's `shared` a value with a destructor
//! (#1332).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    println!("{}", dep::shared(Loud(2)));
}
