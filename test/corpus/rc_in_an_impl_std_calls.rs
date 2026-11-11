// A program's generic impls of std's traits, `Debug for Show<T>` and `Hash
// for Key<T>`, whose methods std calls by their dictionaries, `{:?}`'s, an
// `Option`'s, a `Vec`'s and a `HashSet`'s, where the crate sees no call
// giving their `T` by name: each is given what the program names where it
// makes the dictionary, so an `Rc<T>` of a value that drops is counted, and
// the value is dropped by the last clone, as in a library (#1112).
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

static DROPPED: AtomicU32 = AtomicU32::new(0);

#[derive(Clone)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        DROPPED.fetch_add(self.0, Ordering::Relaxed);
    }
}

struct Show<T>(T);

impl<T: Clone> fmt::Debug for Show<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let first = Rc::new(self.0.clone());
        let other = Rc::clone(&first);
        drop(first);
        write!(f, "shown")?;
        drop(other);
        write!(f, " once")
    }
}

/// Hashed by its tag alone, a copy of its value held in an `Rc` as it is.
struct Key<T>(u32, T);

impl<T: Clone> Hash for Key<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let held = Rc::new(self.1.clone());
        let again = Rc::clone(&held);
        drop(held);
        self.0.hash(state);
        drop(again);
    }
}

impl<T> PartialEq for Key<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Key<T> {}

fn main() {
    let show = Show(Loud(1));
    let shown = format!("{show:?}");
    println!("{shown} {}", DROPPED.load(Ordering::Relaxed));
    drop(show);
    println!("{}", DROPPED.load(Ordering::Relaxed));
    let some = format!("{:?} {:?}", Some(Show(Loud(10))), vec![Show(Loud(100))]);
    println!("{some} {}", DROPPED.load(Ordering::Relaxed));
    let mut keys = HashSet::new();
    keys.insert(Key(1, Loud(1000)));
    println!("{} {}", keys.contains(&Key(1, Loud(0))), DROPPED.load(Ordering::Relaxed));
}
