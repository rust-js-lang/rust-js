// std's `Duration`, as chrono's `TimeDelta` converts to and from one: its
// seconds and nanoseconds, made, asked, added, compared and shown as Rust's.
use std::collections::HashSet;
use std::time::Duration;

fn main() {
    let d = Duration::new(5, 1_500_000_000);
    println!("{} {} {} {}", d.as_secs(), d.subsec_nanos(), d.subsec_millis(), d.subsec_micros());
    println!("{} {} {}", d.as_millis(), d.as_micros(), d.as_nanos());
    let made = [
        Duration::from_secs(90),
        Duration::from_millis(1_250),
        Duration::from_micros(3),
        Duration::from_nanos(1_000_000_007),
        Duration::ZERO,
        Duration::new(0, 1),
    ];
    for m in made {
        println!("{m:?} {} {}", m.is_zero(), m.as_secs());
    }
    println!("{:?} {:?}", Duration::MAX.as_secs(), Duration::MAX.subsec_nanos());
    let a = Duration::from_millis(1_500);
    let b = Duration::from_millis(700);
    println!("{:?} {:?} {:?}", a + b, a - b, a.checked_sub(Duration::from_secs(2)));
    println!("{:?} {:?}", Duration::MAX.checked_add(Duration::new(0, 1)), a.checked_add(b));
    println!("{} {} {} {:?}", a > b, a == Duration::new(1, 500_000_000), a.max(b) == a, a.cmp(&b));
    let mut v = vec![a, b, Duration::ZERO, Duration::from_secs(1)];
    v.sort();
    println!("{v:?}");
    let set: HashSet<Duration> = [a, b, Duration::from_micros(1_500_000)].into_iter().collect();
    println!("{}", set.len());
    println!("{:?} {:?} {:?}", Duration::from_secs(1), Duration::from_millis(10), Duration::new(3, 40));
}
