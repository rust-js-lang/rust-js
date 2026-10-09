// A `OnceCell` or `OnceLock` is a `{ value }` of an `Option`, set once:
// read, set, initialized, taken, and of values that look like `None`.

use std::cell::OnceCell;
use std::sync::OnceLock;

static GREETING: OnceLock<String> = OnceLock::new();

fn greeting() -> &'static str {
    GREETING.get_or_init(|| "hello".to_string())
}

struct Config {
    name: String,
}

fn main() {
    let cell: OnceCell<u32> = OnceCell::new();
    println!("{:?}", cell.get());
    println!("{:?}", cell.set(1));
    println!("{:?}", cell.set(2));
    println!("{:?} {}", cell.get(), cell.get_or_init(|| 3));

    let lazy: OnceCell<String> = OnceCell::new();
    let mut calls = 0;
    for _ in 0..3 {
        let text = lazy.get_or_init(|| {
            calls += 1;
            "made".to_string()
        });
        print!("{text} ");
    }
    println!("{calls}");

    let mut taken: OnceCell<Config> = OnceCell::new();
    println!("{}", taken.take().is_none());
    taken.get_or_init(|| Config { name: "a".to_string() });
    if let Some(config) = taken.get_mut() {
        config.name.push('b');
    }
    println!("{:?}", taken.get().map(|c| c.name.clone()));
    println!("{:?} {}", taken.take().map(|c| c.name), taken.get().is_none());

    let mut count: OnceCell<i32> = OnceCell::new();
    count.set(5).unwrap();
    *count.get_mut().unwrap() += 1;
    println!("{:?}", count.into_inner());

    // `()` and `None` look like `None` in JS: what's set is still there.
    let unit: OnceCell<()> = OnceCell::new();
    println!("{:?} {:?} {:?}", unit.get(), unit.set(()), unit.get());
    let nothing: OnceCell<Option<u8>> = OnceCell::new();
    println!("{:?} {:?}", nothing.get_or_init(|| None), nothing.get());
    println!("{:?}", nothing.set(Some(1)));

    let lock: OnceLock<Vec<u8>> = OnceLock::new();
    println!("{:?}", lock.get());
    lock.get_or_init(|| vec![1, 2]);
    println!("{:?} {:?}", lock.set(vec![3]), lock.get());
    println!("{} {}", greeting(), greeting());
    let shown: OnceCell<Vec<u8>> = OnceCell::new();
    println!("{:?} {shown:?} {shown:#?}", OnceLock::<()>::new());
    shown.set(vec![1]).unwrap();
    println!("{shown:?} {shown:#?}");
    let copy = shown.clone();
    copy.get_or_init(Vec::new);
    println!("{} {}", copy == shown, copy == OnceCell::default());
    println!("{:?} {:?}", unit, nothing.clone());
    println!("{:?}", lock.into_inner());
}
