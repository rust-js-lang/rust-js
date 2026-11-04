// `Default` of the ranges std gives one (#639): `Range<Idx: Default>`'s
// is `start..end` of each one's default, `RangeFull`'s is `..`. Taken out
// of a `&mut` by `mem::take`, given where `T: Default` goes, made by a
// struct's derived `Default`, and of a type parameter's default.

use std::fmt::Debug;
use std::mem;
use std::ops::{Range, RangeFull};
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Default)]
struct Selection {
    rows: Range<usize>,
    columns: Range<i64>,
    everything: RangeFull,
    label: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
struct Point {
    x: i32,
    y: i32,
}

// Each default made in turn, the start's first, as std's derive makes them.
#[derive(Debug, PartialEq, PartialOrd)]
struct Ticket(u32);

static ISSUED: AtomicU32 = AtomicU32::new(0);

impl Default for Ticket {
    fn default() -> Self {
        Ticket(ISSUED.fetch_add(1, Ordering::Relaxed))
    }
}

fn reset<T: Default>(r: &mut T) -> T {
    mem::take(r)
}

fn made<T: Default + Debug>() -> String {
    format!("{:?}", T::default())
}

fn empty_of<T: Default>() -> Range<T> {
    Range::default()
}

fn main() {
    // `mem::take` of a range leaves its default, `0..0`.
    let mut range = 1..4;
    let old = mem::take(&mut range);
    println!("{old:?} {range:?}");
    // Given where `T: Default` goes.
    let mut other = 5..9;
    println!("{:?} {other:?}", reset(&mut other));
    // Taken part way through its steps: what's left, and an empty one.
    let mut stepped = 10..15;
    stepped.next();
    stepped.next_back();
    let rest = mem::take(&mut stepped);
    println!("{:?} {:?}", rest.collect::<Vec<_>>(), stepped.collect::<Vec<_>>());

    // `Range::<T>::default()`, its fields, and its iteration.
    let r = Range::<u8>::default();
    println!("{} {} {} {}", r.start, r.end, r.is_empty(), r.len());
    println!("{:?}", r.collect::<Vec<_>>());
    let mut counted: Range<i32> = Default::default();
    println!("{:?} {:?} {}", counted.next(), counted.size_hint(), counted.count());
    let wide = Range::<u64>::default();
    println!("{wide:?} {}", wide.contains(&0));
    let big = Range::<i128>::default();
    println!("{big:?} {}", big.end - big.start);
    let float = Range::<f64>::default();
    println!("{float:?} {}", float.contains(&0.0));
    let letters = Range::<char>::default();
    println!("{letters:?}");
    println!("{}", letters.count());
    let text = Range::<String>::default();
    println!("{text:?} {}", text.start.is_empty());
    let points = Range::<Point>::default();
    println!("{points:?} {}", points.start == points.end);
    let tickets = Range::<Ticket>::default();
    println!("{tickets:?} {}", tickets.start < tickets.end);
    println!("{:?}", made::<Range<Ticket>>());
    let full = RangeFull::default();
    println!("{full:?}");
    let empty: &[i32] = &[1, 2, 3][Range::default()];
    println!("{empty:?} {:?}", &[4, 5, 6][RangeFull]);

    // A struct's derived `Default` of its ranges.
    let selection = Selection::default();
    println!("{selection:?}");
    println!(
        "{} {} {:?}",
        selection.rows.len(),
        selection.columns.start,
        selection
            .rows
            .clone()
            .chain(selection.columns.start as usize..2)
            .collect::<Vec<_>>()
    );
    println!("{:?} {}", selection.everything, selection.label.len());

    // A generic function's, of its type parameter's default.
    println!("{}", made::<Range<i16>>());
    println!("{}", made::<RangeFull>());
    println!("{}", made::<Range<Point>>());
    println!("{}", made::<Option<Range<u32>>>());
    println!("{:?} {:?}", empty_of::<u16>(), empty_of::<String>());
    let mut ranges: Vec<Range<usize>> = vec![0..3, 4..8];
    let taken: Vec<_> = ranges.iter_mut().map(mem::take).collect();
    println!("{taken:?} {ranges:?}");
    let mut held = Some(2..6);
    let inner = held.take().unwrap_or_default();
    println!("{inner:?} {:?} {:?}", held.clone().unwrap_or_default(), held);
}
