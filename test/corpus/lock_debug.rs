// `{:?}` of a `Mutex` or an `RwLock`: what it holds where it could be locked
// to read it, else `<locked>`, as std's shows it; never poisoned (ADR 0328).
use std::sync::{Arc, Mutex, RwLock};

fn main() {
    let m = Mutex::new(vec![1u32, 2]);
    println!("{m:?}");
    {
        let _held = m.lock().unwrap();
        println!("{m:?}");
    }
    let r = RwLock::new("text");
    {
        let _read = r.read().unwrap();
        println!("{r:?}");
    }
    {
        let _write = r.write().unwrap();
        println!("{r:?}");
    }
    let shared = Arc::new(Mutex::new(5u8));
    println!("{shared:?}");
}
