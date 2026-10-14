//@ run-fail: kept too often
// A struct update whose named field's call panics, after `..p`'s fields
// are counted moved but before they are: what `p` still has is dropped as
// the panic unwinds (ADR 0372).
struct Noisy(u8);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Partial {
    x: Noisy,
    y: Noisy,
}

fn keep(n: Noisy, fail: bool) -> Noisy {
    println!("keep {}", n.0);
    if fail {
        panic!("kept too often");
    }
    Noisy(n.0 + 10)
}

fn swap_y(p: Partial, fail: bool) -> Partial {
    Partial { y: keep(p.y, fail), ..p }
}

fn main() {
    let swapped = swap_y(Partial { x: Noisy(1), y: Noisy(2) }, false);
    println!("swapped {} {}", swapped.x.0, swapped.y.0);
    let failed = swap_y(Partial { x: Noisy(3), y: Noisy(4) }, true);
    println!("never {}", failed.x.0);
}
