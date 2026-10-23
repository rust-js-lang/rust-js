// A panic's payload resumed is its own `Error` again (ADR 0692), but only
// while its message is the one it was thrown with: one changed through
// `downcast_mut` is resumed as a panic of what it is now, as Rust resumes
// the `String` it changed.
use std::panic::{self, AssertUnwindSafe};

fn fails(n: u32) -> u32 {
    if n > 2 {
        panic!("too big: {n}");
    }
    n
}

fn main() {
    let again = panic::catch_unwind(|| match panic::catch_unwind(|| fails(5)) {
        Ok(n) => n,
        Err(mut payload) => {
            if let Some(message) = payload.downcast_mut::<String>() {
                message.push_str(", resumed");
            }
            panic::resume_unwind(payload)
        }
    });
    let message = again.err().and_then(|payload| payload.downcast::<String>().ok());
    println!("{message:?}");
    let kept = panic::catch_unwind(AssertUnwindSafe(|| match panic::catch_unwind(|| fails(4)) {
        Ok(n) => n,
        Err(payload) => panic::resume_unwind(payload),
    }));
    let message = kept.err().and_then(|payload| payload.downcast::<String>().ok());
    println!("{message:?} {:?}", panic::catch_unwind(|| fails(1)).ok());
}
