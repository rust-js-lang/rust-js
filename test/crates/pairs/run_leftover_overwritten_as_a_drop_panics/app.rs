//! A consumer whose items' drop panics for one of them: the library's
//! `next_run`, compiled here from its MIR, overwrites the run's leftover
//! with that one in it, so the write's drop panics, the new leftover is
//! written all the same and the run so far dropped as it unwinds (#1681).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 99 {
            panic!("boom");
        }
    }
}

fn join(a: Loud, b: Loud) -> Result<Loud, (Loud, Loud)> {
    if b.0 == a.0 % 10 + 1 {
        Ok(Loud(a.0 * 10 + b.0))
    } else {
        Err((a, b))
    }
}

pub fn main() {
    let items = vec![Loud(1), Loud(2), Loud(99), Loud(5), Loud(6), Loud(8), Loud(4)];
    let mut runs = dep::Runs::new(items.into_iter());
    let first = runs.next_run(join).map(|run| run.0);
    println!("first {first:?}, leftover {:?}", runs.last().map(|l| l.0));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runs.next_run(join).map(|run| run.0)));
    println!("caught {}", caught.is_err());
    println!("leftover {:?}", runs.last().map(|l| l.0));
}
