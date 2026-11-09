//@ native: wasm32
// `{:?}` of a `SystemTime` and an `Instant` is unix's and wasip1's, its
// seconds and the nanoseconds past them, rust-js's platform's for time,
// each field given the placeholder's options (#1159, ADR 0447).
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
struct Stamped {
    at: SystemTime,
    label: &'static str,
}

fn main() {
    let t = UNIX_EPOCH + Duration::from_millis(1_700_000_000_123);
    println!("{t:?}");
    println!("{:?}", UNIX_EPOCH);
    println!("{:?}", UNIX_EPOCH - Duration::from_millis(1_500));
    println!("{:?}", UNIX_EPOCH - Duration::from_secs(3));
    println!("{t:#?}");
    println!("[{t:12?}] [{t:<12?}] [{t:+?}] [{t:012?}]");
    let before = UNIX_EPOCH - Duration::from_millis(2_500);
    println!("{before:x?} {t:#X?}");
    let stamped = Stamped { at: UNIX_EPOCH + Duration::new(5, 7), label: "five" };
    println!("{stamped:?}");
    println!("{stamped:#?}");
    let start = Instant::now();
    let shown = format!("{start:?}");
    println!("{}", shown.starts_with("Instant { tv_sec: ") && shown.contains(", tv_nsec: ") && shown.ends_with(" }"));
    let later = format!("{:#?}", start + Duration::from_secs(1));
    println!("{}", later.starts_with("Instant {\n    tv_sec: ") && later.ends_with(",\n}"));
}
