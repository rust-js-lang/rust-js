//@ library-refused: another crate may share
// A lock holding a value with a destructor drops it with the lock, as a
// `RefCell` does: alone, shared by an `Arc`, of a `Vec`, in a struct, and
// taken out by `into_inner`.
use std::sync::{Arc, Mutex, RwLock};

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop guard");
    }
}

struct Holder {
    name: &'static str,
    lock: Mutex<Noisy>,
}

fn main() {
    {
        let m = Mutex::new(Guard);
        println!("made");
        drop(m);
        println!("dropped");
    }
    {
        let shared = Arc::new(Mutex::new(Noisy("shared")));
        let other = Arc::clone(&shared);
        drop(shared);
        println!("one left");
        drop(other);
    }
    {
        let l = RwLock::new(vec![Noisy("a"), Noisy("b")]);
        l.write().unwrap().push(Noisy("c"));
        println!("{} held", l.read().unwrap().len());
    }
    {
        let h = Holder { name: "holder", lock: Mutex::new(Noisy("in holder")) };
        println!("{}", h.name);
    }
    {
        let m = Mutex::new(Noisy("old"));
        *m.lock().unwrap() = Noisy("new");
        println!("replaced");
        let w = RwLock::new(Noisy("before"));
        *w.write().unwrap() = Noisy("after");
        println!("written");
    }
    {
        let m = Mutex::new(Noisy("taken"));
        let inner = m.into_inner().unwrap();
        println!("took {}", inner.0);
    }
    println!("end");
}
