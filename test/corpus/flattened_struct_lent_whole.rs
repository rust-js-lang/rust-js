//@ library-differs
// A flattened `Copy` struct lent whole and read whole by value, as a UI
// kit's numeric stepping keeps a `NumberRange` that extends a `StepRange`
// and gives the range to the step functions (#1820): lent, `&range.range`,
// it's the parent's object, which has the parent's keys too; so where it's
// copied, cloned, compared, ordered or hashed, a set's key too, it's its
// own fields alone, `{ min: range.min, max: range.max, step: range.step }`,
// through two levels of flattening as well. A flattened struct never read
// whole keeps its spread, `{ ...margin }`.
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct StepRange {
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct NumberRange {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub range: StepRange,
    pub large_step: Option<f64>,
    pub small_step: Option<f64>,
}

pub fn max_on_grid(range: &StepRange) -> f64 {
    range.max - (range.max - range.min) % range.step
}

fn copied(range: &StepRange) -> StepRange {
    *range
}

fn same(a: &StepRange, b: &StepRange) -> bool {
    a == b
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Grid {
    pub cols: u32,
    pub rows: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sized {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub grid: Grid,
    pub gap: u32,
}

#[derive(Clone, Copy)]
pub struct Board {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub sized: Sized,
    pub name: &'static str,
}

/// Never read whole: its copies keep their spread.
#[derive(Clone, Copy, Debug)]
pub struct Margin {
    pub top: u32,
    pub bottom: u32,
}

#[derive(Clone, Copy)]
pub struct Card {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub margin: Margin,
    pub title: &'static str,
}

fn margin_of(margin: &Margin) -> Margin {
    *margin
}

fn hashed<T: Hash>(t: &T) -> u64 {
    let mut h = DefaultHasher::new();
    t.hash(&mut h);
    h.finish()
}

fn main() {
    let plain = StepRange {
        min: 0.0,
        max: 10.0,
        step: 3.0,
    };
    let range = NumberRange {
        range: plain,
        large_step: Some(5.0),
        small_step: None,
    };
    println!("{}", max_on_grid(&range.range));
    let copy = copied(&range.range);
    let clone = range.range.clone();
    println!("{copy:?} {clone:?} {}", same(&copy, &plain));
    println!(
        "{} {} {:?}",
        same(&range.range, &plain),
        range.range < StepRange { step: 4.0, ..plain },
        range.range.partial_cmp(&copy)
    );
    // A local copy whose step changes, as the kit's fine stepping is.
    let fine = NumberRange {
        range: StepRange {
            step: 1.0,
            ..range.range
        },
        ..range
    };
    println!(
        "{} {:?} {:?}",
        max_on_grid(&fine.range),
        fine.large_step,
        fine.small_step
    );
    let ranges = vec![range.range, fine.range];
    println!("{:?}", ranges);

    // Two levels of flattening, lent and keyed.
    let a = Board {
        sized: Sized {
            grid: Grid { cols: 2, rows: 3 },
            gap: 1,
        },
        name: "a",
    };
    let b = Board {
        sized: Sized {
            grid: Grid { cols: 2, rows: 3 },
            gap: 1,
        },
        name: "b",
    };
    let mut seen = HashSet::new();
    seen.insert(a.sized.grid);
    println!("{} {}", seen.contains(&b.sized.grid), seen.insert(b.sized.grid));
    let mut by: HashMap<Sized, &str> = HashMap::new();
    by.insert(a.sized, a.name);
    println!("{:?} {}", by.get(&b.sized), by.len());
    let mut sorted = BTreeSet::new();
    sorted.insert(b.sized.grid);
    println!(
        "{} {:?}",
        sorted.contains(&a.sized.grid),
        a.sized.grid.cmp(&b.sized.grid)
    );
    println!(
        "{} {}",
        hashed(&a.sized) == hashed(&b.sized),
        hashed(&a.sized.grid) == hashed(&Grid { cols: 2, rows: 3 })
    );
    let sized: Sized = b.sized;
    println!("{sized:?} {} {:?}", sized == a.sized, a.sized.cmp(&sized));

    let margin = Margin { top: 4, bottom: 8 };
    let card = Card { margin, title: "card" };
    let again = margin_of(&margin);
    println!(
        "{:?} {} {} {}",
        again.clone(),
        card.margin.top,
        card.margin.bottom,
        card.title
    );
}
