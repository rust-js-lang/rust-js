// An `Option` of a value with a destructor: `take`, `replace` and `take_if`
// move what it holds out, or in, and give back the old value, which its
// holder drops where Rust's does; `insert` drops the old one, and
// `get_or_insert` the one it's given where it holds one, which is made
// either way; what's left drops when the `Option` does (ADR 0098).
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Slot {
    pending: Option<Noisy>,
}

impl Slot {
    fn flush(&mut self) -> u32 {
        match self.pending.take() {
            Some(n) => n.0,
            None => 0,
        }
    }
}

fn made(n: u32) -> Noisy {
    println!("made {n}");
    Noisy(n)
}

fn main() {
    let mut slot = Slot { pending: Some(Noisy(1)) };
    println!("flushed {}", slot.flush());
    println!("again {}", slot.flush());
    let old = slot.pending.replace(Noisy(2));
    println!("old {}", old.is_some());
    let older = slot.pending.replace(Noisy(3));
    println!("older {:?}", older.as_ref().map(|n| n.0));
    drop(older);
    let kept = slot.pending.take_if(|n| n.0 > 5);
    println!("kept {}", kept.is_none());
    let taken = slot.pending.take_if(|n| n.0 == 3);
    println!("taken {:?} left {}", taken.as_ref().map(|n| n.0), slot.pending.is_none());
    slot.pending = Some(Noisy(4));
    let held = slot.pending.insert(made(5));
    println!("held {}", held.0);
    let same = slot.pending.get_or_insert(made(6));
    println!("same {}", same.0);
    slot.pending = None;
    let fresh = slot.pending.get_or_insert(made(7));
    println!("fresh {}", fresh.0);
    let with = slot.pending.get_or_insert_with(|| made(8));
    println!("with {}", with.0);
    println!("end");
}
