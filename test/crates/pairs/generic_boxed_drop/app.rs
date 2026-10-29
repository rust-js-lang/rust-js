//! Given a value with a destructor: dropped where Rust drops it, with the
//! `Vec` that holds it, once it's printed `held`; and where a library
//! keeps it or not, its drop flag tested where it isn't known (#381).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn main() {
    println!("{}", dep::hold(Loud(1)));
    println!("{}", dep::hold(vec![Loud(2), Loud(3)]));
    let kept = dep::given(Loud(4), true, || println!("checked"));
    println!("kept {}", kept.is_some());
    drop(dep::given(Loud(5), false, || println!("checked")));
    let failed = std::panic::catch_unwind(|| dep::given(Loud(6), false, || panic!("failed")));
    println!("failed {}", failed.is_err());
}
