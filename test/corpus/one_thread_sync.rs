// The sync primitives on the one thread a program has (ADR 1126): `Arc`,
// `Mutex`, `RwLock`, atomics, `OnceLock`, a channel and its `iter()` and
// `try_iter()`, a thread-local's `with` and `try_with`, `thread::panicking()`
// in a drop a panic runs and outside one, and `hint::spin_loop()` (#1176,
// #1285).
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

thread_local! {
    static COUNT: RefCell<u32> = RefCell::new(0);
}

static READY: AtomicBool = AtomicBool::new(false);
static NAME: OnceLock<String> = OnceLock::new();

struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {} panicking={}", self.0, std::thread::panicking());
    }
}

fn main() {
    let shared = Arc::new(Mutex::new(vec![1]));
    let other = Arc::clone(&shared);
    other.lock().unwrap().push(2);
    println!("{:?} {}", shared.lock().unwrap(), Arc::strong_count(&shared));
    let held = shared.try_lock();
    println!("{}", held.is_ok());
    drop(held);
    let rw = RwLock::new(5);
    *rw.write().unwrap() += 1;
    println!("{} {}", *rw.read().unwrap(), rw.try_write().is_ok());
    let n = AtomicU32::new(3);
    n.fetch_add(4, Ordering::SeqCst);
    let old = n.swap(9, Ordering::Relaxed);
    let cas = n.compare_exchange(9, 10, Ordering::AcqRel, Ordering::Acquire);
    println!("{old} {:?} {}", cas, n.load(Ordering::SeqCst));
    let u = AtomicUsize::new(0);
    u.fetch_max(7, Ordering::SeqCst);
    READY.store(true, Ordering::Release);
    println!("{} {}", u.into_inner(), READY.load(Ordering::Acquire));
    NAME.get_or_init(|| "once".to_string());
    println!("{:?} {:?}", NAME.get(), NAME.set("twice".to_string()));
    let (tx, rx) = mpsc::channel();
    tx.send(1).unwrap();
    tx.clone().send(2).unwrap();
    drop(tx);
    println!("{:?}", rx.iter().collect::<Vec<i32>>());
    let (tx, rx) = mpsc::channel::<u8>();
    tx.send(3).unwrap();
    tx.send(4).unwrap();
    let now: Vec<u8> = rx.try_iter().collect();
    tx.send(5).unwrap();
    drop(tx);
    let mut rest = Vec::new();
    for item in rx.iter() {
        rest.push(item);
    }
    println!("{now:?} {rest:?} {:?}", rx.try_recv());
    COUNT.with(|c| *c.borrow_mut() += 1);
    let got = COUNT.try_with(|c| *c.borrow());
    println!("{:?} {}", got.unwrap(), std::thread::panicking());
    std::hint::spin_loop();
    let caught = std::panic::catch_unwind(|| {
        let _n = Noisy("inner");
        panic!("boom");
    });
    println!("{}", caught.is_err());
}
