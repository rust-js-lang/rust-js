// A lock a panic poisons, as std's (ADR 0144, amended; #1438): a `Mutex`'s
// guard or an `RwLock`'s writing one dropped as a panic unwinds poisons it,
// a reading one doesn't, nor one taken as a panic unwinds, in a drop it
// runs; then `lock()`, `read()`, `try_lock()`, `into_inner()` give the
// `PoisonError`, shown as std shows it, `unwrap()` and `expect(..)` of it
// panic, and `clear_poison()` clears it. A failed bounds check poisons as
// a panic does. A `LazyLock` whose init panics is poisoned for good; a
// `OnceLock`'s is tried again.
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{LazyLock, Mutex, OnceLock, RwLock, TryLockError};

static TABLE: LazyLock<Vec<u32>> = LazyLock::new(|| panic!("no table"));

struct Relock<'a>(&'a Mutex<u32>);

impl Drop for Relock<'_> {
    fn drop(&mut self) {
        // Taken as a panic unwinds: its drop poisons nothing.
        let mut held = self.0.lock().unwrap();
        *held += 1;
    }
}

fn message(payload: Box<dyn Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(text) => *text,
        Err(payload) => payload.downcast_ref::<&str>().map(|s| s.to_string()).unwrap_or_default(),
    }
}

fn pick(items: &Mutex<Vec<u32>>, i: usize) -> u32 {
    let held = items.lock().unwrap();
    held[i] + 1
}

fn bump_twice(count: &Mutex<u32>, by: u32) -> u32 {
    let mut held = count.lock().unwrap();
    *held = held.checked_add(by).expect("too many");
    *held = held.checked_add(by).expect("too many");
    *held
}

fn slot(table: &Mutex<[u32; 2]>, i: usize) -> u32 {
    let held = table.lock().unwrap();
    held[i]
}

fn first_over(items: &Mutex<Vec<u32>>, floor: u32) -> Option<u32> {
    let held = items.lock().unwrap();
    for &item in held.iter() {
        if item.checked_sub(floor).expect("below zero") > 0 {
            return Some(item);
        }
    }
    None
}

fn main() {
    let count = Mutex::new(1);
    let caught = panic::catch_unwind(|| {
        let _relock = Relock(&count);
        panic!("relocked");
    });
    println!("relocked {} {} {}", caught.is_err(), count.is_poisoned(), *count.lock().unwrap());

    println!("bumped {}", bump_twice(&count, 3));
    let caught = panic::catch_unwind(|| bump_twice(&count, u32::MAX));
    println!("{:?} {}", caught.map_err(message), count.is_poisoned());
    println!("{:?}", count.lock().map(|held| *held));
    match count.lock() {
        Err(e) => println!("{e} | {e:?} | {}", *e.get_ref()),
        Ok(_) => println!("ok"),
    }
    match count.try_lock() {
        Err(TryLockError::Poisoned(e)) => {
            let shown = e.to_string();
            println!("try {shown} {}", *e.into_inner());
        }
        Err(e) => println!("try {e}"),
        Ok(_) => println!("try ok"),
    }
    let caught = panic::catch_unwind(AssertUnwindSafe(|| *count.lock().expect("lock it")));
    println!("{:?}", caught.map_err(message));
    let caught = panic::catch_unwind(AssertUnwindSafe(|| *count.lock().unwrap()));
    println!("{:?}", caught.map_err(message));
    let recovered = *count.lock().unwrap_or_else(|e| e.into_inner());
    println!("{recovered} {count:?}");
    count.clear_poison();
    println!("{} {count:?}", count.is_poisoned());

    let items = Mutex::new(vec![1, 2, 3]);
    println!("{}", pick(&items, 1));
    let caught = panic::catch_unwind(|| pick(&items, 7));
    println!("{:?} {}", caught.map_err(message), items.is_poisoned());
    items.clear_poison();
    println!("{:?} {:?}", first_over(&items, 1), first_over(&items, 1));
    let caught = panic::catch_unwind(|| first_over(&items, 2));
    println!("{:?} {}", caught.map_err(message), items.is_poisoned());
    items.clear_poison();
    println!("{:?}", first_over(&items, 0));

    let table = Mutex::new([10, 20]);
    let caught = panic::catch_unwind(|| slot(&table, 5));
    println!("{:?} {}", caught.map_err(message), table.is_poisoned());

    let list = RwLock::new(vec![1, 2]);
    let _ = panic::catch_unwind(|| {
        let _first = list.read().unwrap();
        let _second = list.read().unwrap();
        panic!("readers");
    });
    println!("read {} {:?}", list.is_poisoned(), list.read().unwrap());
    let _ = panic::catch_unwind(|| {
        list.write().unwrap().push(3);
        panic!("after a write");
    });
    println!("written {}", list.is_poisoned());
    let _ = panic::catch_unwind(|| {
        let mut writing = list.write().unwrap();
        writing.push(4);
        panic!("writer");
    });
    let read = list.read().map(|items| items.len()).map_err(|e| e.to_string());
    println!("writer {} {read:?} {list:?}", list.is_poisoned());
    list.clear_poison();
    println!("{} {:?}", list.is_poisoned(), list.into_inner());

    let name = Mutex::new(String::from("kept"));
    let _ = panic::catch_unwind(|| {
        let _held = name.lock();
        panic!("held as a Result");
    });
    println!("{:?}", name.into_inner().map_err(|e| e.into_inner()));

    let once: OnceLock<u32> = OnceLock::new();
    let caught = panic::catch_unwind(|| *once.get_or_init(|| panic!("not yet")));
    println!("{} {:?} {}", caught.is_err(), once.get(), once.get_or_init(|| 7));
    for _ in 0..2 {
        let caught = panic::catch_unwind(|| TABLE.len());
        println!("{:?}", caught.map_err(message));
    }
}
