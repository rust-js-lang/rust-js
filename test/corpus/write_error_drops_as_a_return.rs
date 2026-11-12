// A write's `Err` is returned, not unwound: what the writer holds drops as
// its scope ends, an ordinary drop, `thread::panicking()` false in it, and a
// panic in it an ordinary panic, which a `catch_unwind` catches, where JS,
// which throws the `Err`, ran those drops as a panic's cleanup and aborted
// on the drop's panic (#1566).
use std::fmt::{self, Write};
use std::panic::catch_unwind;
use std::thread;

struct Bad;

impl fmt::Debug for Bad {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Err(fmt::Error)
    }
}

/// Says whether it drops as a panic unwinds.
struct Told(u8);

impl Drop for Told {
    fn drop(&mut self) {
        println!("drop {} panicking {}", self.0, thread::panicking());
    }
}

/// Panics as it drops.
struct Loud;

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop Loud");
        panic!("loud drop");
    }
}

struct Holds;

impl fmt::Debug for Holds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _told = Told(1);
        f.debug_list().entry(&1).entry(&Bad).finish()
    }
}

struct HoldsLoud;

impl fmt::Debug for HoldsLoud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _loud = Loud;
        write!(f, "a{:?}", Bad)
    }
}

struct Entries;

impl fmt::Debug for Entries {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let items = vec![Told(2), Told(3)];
        f.debug_list().entries(items.iter().map(|_| Bad)).finish()
    }
}

fn main() {
    let mut s = String::new();
    let r = write!(s, "{:?}", Holds);
    println!("{r:?} {s:?}");
    let caught = catch_unwind(|| {
        let mut s = String::new();
        let r = write!(s, "{:?}", HoldsLoud);
        println!("unreached {r:?}");
    });
    println!("caught {}", caught.is_err());
    let mut s = String::new();
    let r = write!(s, "{:?}", Entries);
    println!("{r:?} {s:?}");
}
