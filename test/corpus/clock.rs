// The clock: `SystemTime` and `Instant`, their nanoseconds, as a
// `Duration` is its own; now, the epoch, the time between two, and a
// `Duration` added or taken (ADR 0447).
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn main() {
    let start = Instant::now();
    let now = SystemTime::now();
    let since = now.duration_since(UNIX_EPOCH).unwrap();
    println!("{}", since.as_secs() > 1_700_000_000);
    let later = UNIX_EPOCH + Duration::from_secs(90);
    println!("{:?}", later.duration_since(UNIX_EPOCH).map(|d| d.as_secs()));
    let err = UNIX_EPOCH.duration_since(later).unwrap_err();
    println!("{} {err} {err:?}", err.duration().as_secs());
    println!("{} {}", later > UNIX_EPOCH, later == SystemTime::UNIX_EPOCH + Duration::from_secs(90));
    println!("{}", later.checked_sub(Duration::from_secs(100)).is_some());
    println!("{}", now.elapsed().is_ok());
    let elapsed = start.elapsed();
    println!("{}", elapsed < Duration::from_secs(5));
    let t2 = start + Duration::from_millis(1500);
    println!("{:?}", t2.duration_since(start));
    println!("{:?}", start.checked_duration_since(t2));
    println!("{:?}", start.saturating_duration_since(t2));
    println!("{:?} {:?}", t2 - start, start - t2);
    let mut t3 = t2;
    t3 -= Duration::from_millis(500);
    t3 += Duration::from_millis(1);
    println!("{:?} {}", t3 - start, t3 < t2);
    println!("{:?}", (t2 - Duration::from_secs(1)).duration_since(start));
}
