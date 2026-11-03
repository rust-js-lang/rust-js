// Iterators of items with a destructor whose values meet (#537): one made
// where it's given to `mem::drop`, `mem::replace` or generic code is std's
// struct, as the one a variable holds is; what's moved out is its own,
// dropping what it has left where it's dropped; what `*it = next`
// replaces drops what it had left there, and what's written there drops
// its own where the variable does, as Rust drops each.
use std::mem;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn seen(l: Loud) -> Loud {
    println!("seen {}", l.0);
    l
}

fn refill<I: Iterator<Item = Loud>>(it: &mut I, next: I) -> I {
    mem::replace(it, next)
}

fn put<I: Iterator<Item = Loud>>(it: &mut I, next: I) {
    *it = next;
    println!("put");
}

fn main() {
    let items = vec![Loud(1), Loud(2), Loud(3)];
    drop(items.into_iter().map(seen));
    println!("dropped");

    let mut it = vec![Loud(4), Loud(5)].into_iter().map(seen);
    let first = it.next();
    let old = mem::replace(&mut it, vec![Loud(6), Loud(7)].into_iter().map(seen));
    println!("replaced");
    drop(old);
    println!("old dropped");
    drop(first);
    println!("{:?}", it.next().map(|l| l.0));

    let mut back = vec![Loud(10), Loud(11), Loud(12)].into_iter().rev();
    println!("{:?}", back.next().map(|l| l.0));
    let old = refill(&mut back, vec![Loud(13), Loud(14)].into_iter().rev());
    println!("{:?}", back.len());
    drop(old);
    println!("old dropped");
    println!("{:?}", back.next().map(|l| l.0));
    drop(back);

    let mut back = vec![Loud(20), Loud(21), Loud(22)].into_iter().rev();
    println!("{:?}", back.next().map(|l| l.0));
    put(&mut back, vec![Loud(23), Loud(24)].into_iter().rev());
    println!("{:?}", back.next().map(|l| l.0));
    let mut mapped = vec![Loud(25), Loud(26)].into_iter().map(seen);
    println!("{:?}", mapped.next().map(|l| l.0));
    put(&mut mapped, vec![Loud(27), Loud(28)].into_iter().map(seen));
    println!("{:?}", mapped.next().map(|l| l.0));
    let mut taken = vec![Loud(29), Loud(30)].into_iter().take(2);
    taken.next();
    let mut other = vec![Loud(31)].into_iter().take(1);
    mem::swap(&mut taken, &mut other);
    println!("{:?}", other.next().map(|l| l.0));
    println!("end");
}
