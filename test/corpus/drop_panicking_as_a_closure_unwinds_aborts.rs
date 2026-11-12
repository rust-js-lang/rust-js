//@ run-abort
// A drop that panics while a closure's `assert!` panic unwinds, of the
// argument the closure was given, aborts the process, as Rust's does: a
// panic in a destructor during cleanup (#1606).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 2 {
            panic!("drop of 2");
        }
    }
}

fn main() {
    let total = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        vec![D(1), D(2)]
            .into_iter()
            .flat_map(|x| {
                assert!(x.0 != 2, "flat_map");
                vec![x]
            })
            .fold(0, |a, d| a + d.0)
    }));
    println!("never {}", total.is_err());
}
