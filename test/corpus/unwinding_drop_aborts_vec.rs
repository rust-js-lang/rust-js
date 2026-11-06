//@ run-abort
// An item of a `Vec` whose drop panics while a panic unwinds aborts (#847):
// the items after it are dropped as its own panic unwinds, then the process
// aborts, and `catch_unwind` never returns.
use std::panic;

struct Bomb(&'static str);

impl Drop for Bomb {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == "bomb" {
            panic!("bomb went off");
        }
    }
}

fn main() {
    let r = panic::catch_unwind(|| {
        let kept = vec![Bomb("x"), Bomb("y")];
        drop(kept);
        let _items = vec![Bomb("a"), Bomb("bomb"), Bomb("c")];
        let n: Option<u32> = None;
        n.expect("nothing there");
    });
    println!("never {}", r.is_err());
}
