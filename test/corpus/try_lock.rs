// `try_lock`, `try_read` and `try_write`: on one thread, `Ok` of a guard,
// or `Err(WouldBlock)` while the thread holds it, where `lock()` would
// deadlock.
use std::sync::{Mutex, RwLock, TryLockError};

fn main() {
    let m = Mutex::new(5);
    {
        let mut g = m.try_lock().unwrap();
        *g += 1;
        match m.try_lock() {
            Err(TryLockError::WouldBlock) => println!("busy"),
            Err(TryLockError::Poisoned(_)) => println!("poisoned"),
            Ok(_) => println!("free"),
        }
        println!("{}", m.try_lock().is_err());
    }
    if let Ok(g) = m.try_lock() {
        println!("{}", *g);
    }

    let l = RwLock::new(String::from("a"));
    {
        let r1 = l.try_read().unwrap();
        let r2 = l.try_read().unwrap();
        println!("{} {} {}", *r1, *r2, l.try_write().is_err());
    }
    l.try_write().unwrap().push('b');
    {
        let w = l.write().unwrap();
        let e = l.try_read().unwrap_err();
        println!("{e} {e:?} {}", w.len());
    }
    println!("{}", l.read().unwrap());
}
