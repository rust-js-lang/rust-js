//@ native: wasm32
//@ library-differs
// std's types whose `Hash` rust-js writes as std's does, compared with
// native wasm32's raw `DefaultHasher` values: a `Duration`, `SystemTime` and
// `Instant`, their seconds then the nanoseconds under one, a `SystemTime`'s
// before the epoch floored; an `OsString`'s bytes; a `Path`'s and a
// `PathBuf`'s components, std's algorithm, `.` parts, repeated and trailing
// separators, root and empty; a `Discriminant`, `PhantomPinned` and a
// `Layout`; a `TypeId`'s hash equal where it is, its value unspecified; and
// each in a tuple key beside one whose `Hash` counts its calls (#1447).
use std::alloc::Layout;
use std::any::TypeId;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomPinned;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

thread_local! {
    static HASHED: Cell<u32> = const { Cell::new(0) };
}

/// A key whose `Hash` counts its calls.
#[derive(PartialEq, Eq, Debug)]
struct Counted(u32);

impl Hash for Counted {
    fn hash<H: Hasher>(&self, state: &mut H) {
        HASHED.set(HASHED.get() + 1);
        self.0.hash(state);
    }
}

fn hashed<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut state = DefaultHasher::new();
    value.hash(&mut state);
    state.finish()
}

fn main() {
    let durations = [
        Duration::ZERO,
        Duration::from_millis(1500),
        Duration::new(u64::MAX, 999_999_999),
        Duration::from_nanos(7),
    ];
    for d in durations {
        println!("{d:?} {}", hashed(&d));
    }
    let mut both = DefaultHasher::new();
    both.write_u64(1);
    both.write_u32(500_000_000);
    println!("{}", both.finish() == hashed(&Duration::from_millis(1500)));
    let later = UNIX_EPOCH + Duration::new(1_700_000_000, 123);
    let before = UNIX_EPOCH - Duration::from_millis(1500);
    println!("{} {} {}", hashed(&UNIX_EPOCH), hashed(&later), hashed(&before));
    let start = Instant::now();
    let same = start;
    println!(
        "{} {}",
        hashed(&start) == hashed(&same),
        hashed(&start) == hashed(&(start + Duration::from_nanos(1)))
    );
    let before = HASHED.get();
    let keys: HashSet<(Duration, Counted)> = [
        (Duration::from_secs(1), Counted(1)),
        (Duration::from_secs(1), Counted(1)),
        (Duration::from_secs(2), Counted(1)),
    ]
    .into_iter()
    .collect();
    println!(
        "{} {} hashed {}",
        keys.len(),
        keys.contains(&(Duration::from_secs(2), Counted(1))),
        HASHED.get() - before
    );
    let times: HashMap<(SystemTime, Counted), &str> = [((later, Counted(3)), "later")].into_iter().collect();
    println!("{:?}", times.get(&(later, Counted(3))));

    let paths = [
        "", "/", "//", "a", "ab", "a/b", "a//b", "a/b/", "a/./b", "./a", "a/.", ".", "./", "..", "a/..", "/a/b",
        "a/.b", "a/./", "a/.//b", "a/b/.", "/.", "./.", "é/ü", "a/...", "a/.x/./y",
    ];
    for p in paths {
        println!(
            "{p:?} {} {}",
            hashed(Path::new(p)),
            hashed(&PathBuf::from(p)) == hashed(Path::new(p))
        );
    }
    println!("{}", hashed(Path::new("a/b")) == hashed(Path::new("a//b/")));
    println!(
        "{} {}",
        hashed(&TypeId::of::<u8>()) == hashed(&TypeId::of::<u8>()),
        hashed(&TypeId::of::<u8>()) == hashed(&TypeId::of::<u16>())
    );
    let keys: HashSet<(PathBuf, Counted)> = [
        (PathBuf::from("a/b"), Counted(4)),
        (PathBuf::from("a//b"), Counted(4)),
        (PathBuf::from("a/c"), Counted(4)),
    ]
    .into_iter()
    .collect();
    println!("{}", keys.len());

    println!(
        "{} {} {}",
        hashed(&OsString::from("aé")),
        hashed(&std::mem::discriminant(&Some(1))),
        hashed(&PhantomPinned)
    );
    println!(
        "{} {}",
        hashed(&Layout::new::<u64>()),
        hashed(&Layout::from_size_align(24, 8).unwrap())
    );
}
