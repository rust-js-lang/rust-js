// A backtrace is disabled unless forced (ADR 0568), as Rust's is unless
// `RUST_BACKTRACE` asks, which JS has no environment to: `capture()` and
// `disabled()` give one that shows as `disabled backtrace`, `force_capture()`
// one with frames. Native Rust is run with `RUST_BACKTRACE` unset, as the
// suite does.
use std::backtrace::{Backtrace, BacktraceStatus};

struct Failure {
    message: &'static str,
    backtrace: Backtrace,
}

impl Failure {
    fn new(message: &'static str) -> Failure {
        Failure {
            message,
            backtrace: Backtrace::capture(),
        }
    }
}

fn main() {
    let failure = Failure::new("it broke");
    let captured = failure.backtrace.status() == BacktraceStatus::Captured;
    println!("{} {} {}", failure.message, captured, failure.backtrace);
    println!("{:?} {:?}", failure.backtrace, failure.backtrace.status());

    let off = Backtrace::disabled();
    println!("{} {}", off.status() == BacktraceStatus::Disabled, off);

    let forced = Backtrace::force_capture();
    println!(
        "{} {}",
        forced.status() == BacktraceStatus::Captured,
        forced.to_string().is_empty()
    );
}
