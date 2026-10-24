// A fieldless enum under a `#[repr(..)]` of an integer is its number in
// JS, as rustc numbers its variants, `ArrayKind::Empty` `0` (ADR 0722):
// tested, matched, compared, ordered, shown, cast, hashed and kept in a
// map as std's would, its derives each Rust's. One without a `repr` is
// its variant's name, as ever.

use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
enum ArrayKind {
    Empty = 0,
    Complex = 1,
    Primitive = 2,
    Entries = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(i8)]
enum Level {
    Low = -1,
    #[default]
    Mid = 7,
    High = 3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u16)]
enum Counted {
    First,
    Second,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Named {
    Plain,
    Other,
}

fn kind_of(n: usize, entries: bool) -> ArrayKind {
    let mut kind = ArrayKind::Empty;
    if n > 0 {
        kind = if entries { ArrayKind::Entries } else { ArrayKind::Primitive };
    }
    if kind != ArrayKind::Empty && n > 5 {
        return ArrayKind::Complex;
    }
    kind
}

fn describe(kind: ArrayKind) -> &'static str {
    match kind {
        ArrayKind::Empty => "empty",
        ArrayKind::Complex => "complex",
        ArrayKind::Primitive => "primitive",
        ArrayKind::Entries => "entries",
    }
}

fn level_name(level: Level) -> &'static str {
    match level {
        Level::Low => "Low",
        Level::Mid => "Mid",
        Level::High => "High",
    }
}

fn main() {
    let kinds = [kind_of(0, false), kind_of(2, false), kind_of(2, true), kind_of(9, true)];
    for kind in kinds {
        println!("{kind:?} {} {} {}", describe(kind), kind as u8, kind as i32 + 1);
    }
    println!("{} {}", kinds[0] == ArrayKind::Empty, kinds[1] < kinds[2]);
    let mut sorted = kinds.to_vec();
    sorted.sort();
    println!("{sorted:?} {:?}", sorted.iter().max());
    let mut levels = vec![Level::Mid, Level::High, Level::Low];
    levels.sort();
    println!("{levels:?} {:?} {}", Level::default(), Level::Low as i8);
    println!("{}", levels.iter().map(|&l| level_name(l)).collect::<Vec<_>>().join(","));
    println!("{:?} {} {}", Counted::Second, Counted::First as u16, Counted::Second == Counted::First);
    println!("{:?} {}", Named::Other, Named::Plain as u8);
    let mut counts: HashMap<ArrayKind, u32> = HashMap::new();
    for kind in [kinds, [kinds[1], kinds[2], kinds[1], kinds[2]]].concat() {
        *counts.entry(kind).or_default() += 1;
    }
    let mut counted: Vec<_> = counts.into_iter().collect();
    counted.sort();
    println!("{counted:?}");
    let seen: HashSet<ArrayKind> = kinds.into_iter().collect();
    println!("{} {}", seen.contains(&ArrayKind::Primitive), seen.len());
    let maybe = [None, Some(ArrayKind::Empty), Some(ArrayKind::Entries)];
    for kind in maybe {
        println!("{kind:?} {} {}", kind.is_some(), kind == Some(ArrayKind::Empty));
    }
}
