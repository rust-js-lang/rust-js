// A `Debug` builder's `entries` of an iterator whose step panics, in a
// writer that neither prints nor fails, shown only in a `format!`, which a
// `catch_unwind` catches: its text is made whole, as it was, and what's left
// of the iterator, the `Vec` chained after the step that panics, is dropped
// as the panic unwinds, as std's `for entry in entries` drops it (#1141);
// and a writer that panics part-way, shown only in a `format!`.
// Nothing it wrote reaches a line, so it doesn't write through a sink
// (#1496, #1533).
use std::fmt;
use std::panic::catch_unwind;
use std::sync::atomic::{AtomicUsize, Ordering};

static DROPPED: AtomicUsize = AtomicUsize::new(0);

struct Counted(u32);

impl Drop for Counted {
    fn drop(&mut self) {
        DROPPED.fetch_add(1, Ordering::SeqCst);
    }
}

impl fmt::Debug for Counted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "C{}", self.0)
    }
}

struct Steps(u32);

impl Iterator for Steps {
    type Item = Counted;

    fn next(&mut self) -> Option<Counted> {
        self.0 += 1;
        assert!(self.0 < 3, "step {}", self.0);
        Some(Counted(self.0))
    }
}

struct Listed(u32);

impl fmt::Debug for Listed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let held = vec![Counted(10), Counted(11)];
        f.debug_list().entries(Steps(self.0).chain(held)).finish()
    }
}

struct Quiet(u32);

impl fmt::Debug for Quiet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Q")?;
        if self.0 > 5 {
            panic!("quiet");
        }
        write!(f, "{}", self.0)
    }
}

fn main() {
    let r = catch_unwind(|| format!("{:?}", Listed(0)));
    println!("{} {}", r.is_err(), DROPPED.load(Ordering::SeqCst));
    let shown = catch_unwind(|| format!("{:?}", Listed(5)));
    println!("{} {}", shown.is_err(), DROPPED.load(Ordering::SeqCst));
    let quiet = catch_unwind(|| format!("{:?}", Quiet(9)));
    let text = format!("{:?}", Quiet(1));
    println!("{} {text}", quiet.is_err());
}
