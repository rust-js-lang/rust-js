//@ run-abort
// A drop that panics while a panic unwinds aborts the process, as Rust does
// (#847, ADR 0660 amended): `catch_unwind` never returns, and nothing after
// runs. A drop that catches its own panic as it unwinds goes on.
use std::panic;

struct Bomb(&'static str);

impl Drop for Bomb {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0.starts_with("bomb") {
            panic!("{} went off", self.0);
        }
    }
}

struct Careful(&'static str);

impl Drop for Careful {
    fn drop(&mut self) {
        let caught = panic::catch_unwind(|| panic!("inside {}", self.0));
        println!("drop {}: caught its own {}", self.0, caught.is_err());
    }
}

fn main() {
    let r = panic::catch_unwind(|| {
        let _a = Careful("careful");
        panic!("first");
    });
    println!("caught {}", r.is_err());
    let r = panic::catch_unwind(|| {
        let _a = Bomb("a");
        let _b = Bomb("bomb");
        let _c = Bomb("c");
        panic!("second");
    });
    println!("never {}", r.is_err());
}
