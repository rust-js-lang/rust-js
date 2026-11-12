// A chain, a map or a filter of the crate's iterator with a `Drop`, cloned
// apart (ADR 0525) where the crate clones one, closes each source to drop
// it, as the adapter not cloned apart does: each temporary's at its
// statement's end, a clone's where it ends, a held one's at its scope's
// end (#1506).
#[derive(Clone)]
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        self.next().map(|n| n + 100)
    }
}

impl Drop for Countdown {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let counted = Countdown(3).chain(Countdown(2)).map(|n| n * 3).next_back();
    println!("{counted:?}");
    let plain = Countdown(4).chain(Countdown(5)).next_back();
    println!("{plain:?}");
    let filtered = Countdown(6).chain(Countdown(7)).filter(|n| n % 2 == 0).next_back();
    println!("{filtered:?}");
    let front = Countdown(3).chain(Countdown(2)).map(|n| n * 3).next();
    println!("{front:?}");
    let both = Countdown(2).chain(Countdown(1));
    let again: Vec<u32> = both.clone().rev().collect();
    println!("{again:?} {}", both.count());
    let mapped = Countdown(3).map(|n| n + 1);
    let copy: Vec<u32> = mapped.clone().collect();
    println!("{copy:?}");
    let kept = Countdown(4).filter(|n| n % 2 == 1);
    let ahead = kept.clone().next();
    println!("{ahead:?} {}", kept.count());
    let held = mapped.rev().next();
    println!("{held:?} end");
}
