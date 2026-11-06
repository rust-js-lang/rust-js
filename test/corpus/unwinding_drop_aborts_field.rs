//@ run-abort
// A field's drop that panics while a panic unwinds aborts (#847): the
// fields after it are dropped as its own panic unwinds, then the process
// aborts, with no `catch_unwind` anywhere; nothing after runs.
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Bomb(u32);

impl Drop for Bomb {
    fn drop(&mut self) {
        println!("bomb {}", self.0);
        if self.0 > 1 {
            panic!("bomb {} went off", self.0);
        }
    }
}

struct Parts {
    first: Noisy,
    bomb: Bomb,
    last: Noisy,
}

fn held(n: u32) -> u32 {
    let parts = Parts {
        first: Noisy("first"),
        bomb: Bomb(n),
        last: Noisy("last"),
    };
    if n > 0 {
        panic!("unwinding with {}", parts.bomb.0);
    }
    n
}

fn main() {
    println!("held {}", held(0));
    held(2);
    println!("never");
}
