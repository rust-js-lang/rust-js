// `sort_unstable_by`, `sort_unstable_by_key`, a type's own `Ord` and
// `select_nth_unstable*` leave items that compare equal in the order std's
// algorithms leave them, which isn't a stable sort's: each length takes
// another path, insertion sort, a small sort, quicksort with its pivots.

#[derive(Debug, Clone, Copy)]
struct Wide {
    key: u32,
    pad: [u32; 30],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Loose(u32, u32);

// Ordered by the first number alone.
impl PartialOrd for Loose {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Loose {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

// Of 8 bytes, `Freeze` but not `Copy`: std's general small sort, `sort8_stable`.
#[derive(Debug, Clone)]
struct Tag(u32, u32);

// Of 32 bytes: the general small sort of `sort4_stable`s.
#[derive(Debug, Clone)]
struct Mid {
    key: u32,
    rest: [u32; 7],
}

// Not `Freeze`: the fallback, insertion sort.
#[derive(Debug, Clone)]
struct Counted(std::cell::Cell<u32>, u32);

fn order(items: &[(u32, u32)]) -> String {
    items.iter().map(|p| p.1.to_string()).collect::<Vec<_>>().join(",")
}

fn main() {
    for n in [10u32, 18, 21, 33, 64, 100, 300] {
        let base: Vec<(u32, u32)> = (0..n).map(|i| ((i * 7919) % 5, i)).collect();
        let mut by_key = base.clone();
        by_key.sort_unstable_by_key(|p| p.0);
        let mut by = base.clone();
        by.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        println!("{n}: {}", order(&by_key));
        println!("{n}: {}", order(&by));

        let mut loose: Vec<Loose> = base.iter().map(|p| Loose(p.0, p.1)).collect();
        loose.sort_unstable();
        println!("{n}: {}", loose.iter().map(|l| l.1.to_string()).collect::<Vec<_>>().join(","));

        let mut wide: Vec<Wide> = base.iter().map(|p| Wide { key: p.0, pad: [p.1; 30] }).collect();
        wide.sort_unstable_by_key(|w| w.key);
        println!("{n}: {}", wide.iter().map(|w| w.pad[0].to_string()).collect::<Vec<_>>().join(","));

        // How many times std asks, which each small sort asks as many as it does.
        let asked = std::cell::Cell::new(0);
        let mut tags: Vec<Tag> = base.iter().map(|p| Tag(p.0, p.1)).collect();
        tags.sort_unstable_by(|a, b| {
            asked.set(asked.get() + 1);
            a.0.cmp(&b.0)
        });
        println!("{n}: {} after {}", tags.iter().map(|t| t.1.to_string()).collect::<Vec<_>>().join(","), asked.get());
        let mut mids: Vec<Mid> = base.iter().map(|p| Mid { key: p.0, rest: [p.1; 7] }).collect();
        mids.sort_unstable_by(|a, b| a.key.cmp(&b.key));
        println!("{n}: {}", mids.iter().map(|m| m.rest[0].to_string()).collect::<Vec<_>>().join(","));
        let mut counted: Vec<Counted> = base.iter().map(|p| Counted(std::cell::Cell::new(p.0), p.1)).collect();
        counted.sort_unstable_by_key(|c| c.0.get());
        println!("{n}: {}", counted.iter().map(|c| c.1.to_string()).collect::<Vec<_>>().join(","));

        // In order already, it's left as it is; in reverse, reversed.
        let mut again = by_key.clone();
        again.sort_unstable_by_key(|p| p.0);
        let mut down: Vec<(u32, u32)> = (0..n).rev().map(|i| (i, i)).collect();
        down.sort_unstable_by_key(|p| p.0);
        println!("{n}: {} {}", order(&again), order(&down[..3]));

        let mut picked = base.clone();
        let middle = picked.len() / 2;
        let nth = *picked.select_nth_unstable_by_key(middle, |p| p.0).1;
        println!("{n}: {:?} {}", nth, order(&picked));
        let mut picked = base.clone();
        picked.select_nth_unstable_by(1, |a, b| b.0.cmp(&a.0));
        println!("{n}: {}", order(&picked));
        let mut numbers: Vec<u32> = base.iter().map(|p| (p.1 * 31) % 17).collect();
        *numbers.select_nth_unstable(middle).1 += 100;
        let mut ends = numbers.clone();
        let last = ends.len() - 1;
        ends.select_nth_unstable(last);
        ends.select_nth_unstable(0);
        println!("{n}: {:?} {:?}", numbers, ends);
    }
}
