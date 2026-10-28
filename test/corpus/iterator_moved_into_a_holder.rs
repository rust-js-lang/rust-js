// An iterator of what has a destructor moved into a call whose value the
// body drops, a type of the crate's own that holds it, `Holder<Take<..>>`:
// what it has left is dropped with that value, at its scope's end, not as
// its `take` reaches its limit (#243).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn three(base: u32) -> Vec<Loud> {
    vec![Loud(base), Loud(base + 1), Loud(base + 2)]
}

struct Holder<I> {
    it: I,
}

fn wrap<I>(it: I) -> Holder<I> {
    Holder { it }
}

fn main() {
    let mut kept = wrap(three(100).into_iter().take(2));
    let first = kept.it.next().map(|l| l.0);
    let second = kept.it.next().map(|l| l.0);
    let past = kept.it.next().map(|l| l.0);
    println!("kept {first:?} {second:?} {past:?}");
    let mut twice = wrap(wrap(three(110).into_iter().take(1)));
    let only = twice.it.it.next().map(|l| l.0);
    let after = twice.it.it.next().map(|l| l.0);
    println!("twice {only:?} {after:?}");
}
