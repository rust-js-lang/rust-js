// A type parameter given as it is (ADR 0411), a `Vec`'s `IntoIter` of
// items with a destructor, which generic code skips, steps by and takes the
// `nth` of: each item it passes over is dropped as its `Vec` drops it, as
// one slice, front to back, by the drop of each its caller gives (#718).
// They were never dropped, as the drop `held_drop_function` gave it hid
// each item's own.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Held<I>(I);

impl<I: DoubleEndedIterator<Item = D>> Held<I> {
    fn mapped_nth(self) -> Option<u32> {
        self.0.map(|d| d.0).nth(1)
    }

    fn skipped(self) -> Option<u32> {
        self.0.skip(1).map(|d| d.0).next()
    }

    fn stepped(self) -> Vec<u32> {
        self.0.step_by(2).map(|d| d.0).collect()
    }
}

fn ds() -> std::vec::IntoIter<D> {
    (1..5).map(D).collect::<Vec<_>>().into_iter()
}

fn main() {
    println!("{:?}", Held(ds()).mapped_nth());
    println!("{:?}", Held(ds()).skipped());
    println!("{:?}", Held(ds()).stepped());
    println!("end");
}
