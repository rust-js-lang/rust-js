//! A library's generic function the crate compiles itself, of MIR alone,
//! that boxes its own closure, holding a guard whose drop prints, as a
//! `dyn FnMut` the crate's own code never names: its drop runs as native's
//! does.
use std::fmt::Display;
struct Guard(&'static str);
impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
pub fn shown<T: Display>(t: T) -> String {
    let g = Guard("closure");
    let mut f: Box<dyn FnMut() -> String> = Box::new(move || format!("[{t}] {}", g.0));
    let once = f();
    let twice = f();
    format!("{once} {twice}")
}
