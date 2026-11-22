// `clone_from_slice` of another length panics as std's does: of items std
// clones as their bits, `TrivialClone`'s (an integer, a `#[derive(Clone,
// Copy)]` type, a tuple of them), std's specialization copies them, with
// `copy_from_slice`'s message; of others, a hand-written `Clone` of a
// `Copy` type too, its own assertion's.
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};

#[derive(Clone, Copy, Debug)]
struct Point(i32, i32);

#[derive(Copy, Debug)]
struct Noisy(i32);

impl Clone for Noisy {
    #[allow(clippy::non_canonical_clone_impl)]
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        Noisy(self.0)
    }
}

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check(what: &str, f: impl FnOnce()) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(()) => println!("{what}: cloned"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn main() {
    let mut numbers = [1, 2, 3];
    check("numbers", || numbers.clone_from_slice(&[4, 5]));
    let mut points = [Point(0, 0); 2];
    check("points", || points.clone_from_slice(&[Point(1, 2)]));
    let mut pairs = [(1, 'x'); 2];
    check("pairs", || pairs.clone_from_slice(&[(2, 'y'); 3]));
    let mut noisy = [Noisy(0)];
    check("noisy", || noisy.clone_from_slice(&[]));
    let mut words = [String::from("a")];
    check("words", || words.clone_from_slice(&[]));
    let mut same = [Point(0, 0); 2];
    check("same", || same.clone_from_slice(&[Point(3, 4), Point(5, 6)]));
    println!("{same:?}");
}
