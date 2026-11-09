//! Each closure dropped as Rust drops it (#1051).
use dep::fold_with;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    let a = Loud("f");
    let b = Loud("g");
    let total = fold_with(
        1..4,
        move |x| {
            let _ = &a;
            x * 2
        },
        0,
        move |s, x| {
            let _ = &b;
            s * 10 + x
        },
    );
    println!("{total}");
}
