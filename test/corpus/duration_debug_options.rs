// A placeholder's options of a `Duration`'s `{:?}` are applied by std's own
// rules: its precision's digits, a `+`, then its width, padded by the fill,
// to the left where it's not aligned, and never by zeros; a
// `SystemTimeError`'s `{:?}` gives them to its `Duration`, as does a derived
// `Debug` of a struct that holds one (ADRs 0188, 0447).
use std::time::{Duration, UNIX_EPOCH};

#[derive(Debug)]
struct Held {
    took: Duration,
}

fn main() {
    let d = Duration::from_millis(1500);
    println!("[{d:10?}] [{d:>10?}] [{d:^10?}] [{d:+?}] [{d:010?}] [{d:*<10.3?}] [{d:#?}] [{d:.0?}]");
    let small = Duration::from_micros(1_234);
    println!("[{small:8?}] [{small:>9.1?}] [{:-^9?}]", Duration::from_nanos(7));
    let held = Held { took: d };
    println!("[{held:10?}] [{held:+?}]");
    println!("{held:#12?}");
    println!("{}", held.took.as_millis());
    let error = UNIX_EPOCH.duration_since(UNIX_EPOCH + d).unwrap_err();
    println!("[{error:>20?}] [{error:?}] [{error:.2?}] [{error:<8?}]");
    println!("{error:#?}");
    println!("{error:#8?}");
}
