//! A library's generic function the crate compiles itself, of MIR alone,
//! holding a type of the library's the crate's own code never names, whose
//! drop prints: the guard's drop runs as native's does.
use std::fmt::Display;
struct Guard(&'static str);
impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
pub fn shown<T: Display>(t: T) -> String {
    let _g = Guard("shown");
    format!("[{t}]")
}
