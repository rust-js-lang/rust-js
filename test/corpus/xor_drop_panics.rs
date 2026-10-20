//@ run-fail: boom 1
// `xor` of two `Some`s drops both, its own first: where that one's drop
// panics, the other's is dropped still, as Rust's unwinding drops it (ADR
// 0502).
struct Loud(u8, bool);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.1 {
            panic!("boom {}", self.0);
        }
    }
}

fn main() {
    let calm = Some(Loud(5, false)).xor(Some(Loud(6, false)));
    println!("calm {}", calm.is_none());
    let x = Some(Loud(1, true)).xor(Some(Loud(2, false)));
    println!("unreached {}", x.is_none());
}
