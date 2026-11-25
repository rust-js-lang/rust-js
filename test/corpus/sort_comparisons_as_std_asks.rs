// A sort's comparator is asked what std's asks, in std's order (ADR 0036):
// of each length about std's thresholds, insertion's 20, the small sorts'
// and the merges', and each of five patterns, the calls' count and a hash
// of each pair compared, stable and unstable, of a small `Freeze` item, a
// large one and one that isn't, `Cell`'s, each std's own small sort; and
// an item with a destructor, each dropped once. Probed against native when
// #2588 asked whether its count of a hash table's entries was the model's.
use std::cell::Cell;

struct Trace {
    calls: u64,
    hash: u64,
}

impl Trace {
    fn new() -> Self {
        Trace {
            calls: 0,
            hash: 0xcbf2_9ce4_8422_2325,
        }
    }

    fn saw(&mut self, a: u64, b: u64) {
        self.calls += 1;
        for x in [a, b] {
            self.hash = (self.hash ^ x).wrapping_mul(0x0100_0000_01b3);
        }
    }
}

thread_local! {
    static DROPS: Cell<usize> = const { Cell::new(0) };
}

struct Dropped(u64);

impl Drop for Dropped {
    fn drop(&mut self) {
        DROPS.with(|drops| drops.set(drops.get() + 1));
    }
}

fn pattern(len: usize, kind: u32) -> Vec<u64> {
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d ^ (len as u64) ^ (u64::from(kind) << 32);
    let mut next = move || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        seed >> 33
    };
    (0..len as u64)
        .map(|i| match kind {
            0 => i,
            1 => len as u64 - i,
            2 => next() % 1000,
            3 => next() % 4,
            _ => i % 7,
        })
        .collect()
}

fn main() {
    let lens: Vec<usize> = (0..=22)
        .chain([31, 32, 33, 47, 48, 63, 64, 65, 128, 129, 300])
        .collect();
    for &len in &lens {
        for kind in 0..5 {
            let base = pattern(len, kind);
            let mut t = Trace::new();
            let mut small = base.clone();
            small.sort_by(|a, b| {
                t.saw(*a, *b);
                a.cmp(b)
            });
            println!("stable small {len} {kind} {} {:x}", t.calls, t.hash);
            let mut t = Trace::new();
            let mut large: Vec<[u64; 4]> = base.iter().map(|&x| [x, 0, 0, 0]).collect();
            large.sort_by(|a, b| {
                t.saw(a[0], b[0]);
                a[0].cmp(&b[0])
            });
            println!("stable large {len} {kind} {} {:x}", t.calls, t.hash);
            let mut t = Trace::new();
            let mut cells: Vec<Cell<u64>> = base.iter().map(|&x| Cell::new(x)).collect();
            cells.sort_by(|a, b| {
                t.saw(a.get(), b.get());
                a.get().cmp(&b.get())
            });
            println!("stable cell {len} {kind} {} {:x}", t.calls, t.hash);
            let mut t = Trace::new();
            let mut small = base.clone();
            small.sort_unstable_by(|a, b| {
                t.saw(*a, *b);
                a.cmp(b)
            });
            println!("unstable small {len} {kind} {} {:x}", t.calls, t.hash);
            let mut t = Trace::new();
            let mut cells: Vec<Cell<u64>> = base.iter().map(|&x| Cell::new(x)).collect();
            cells.sort_unstable_by(|a, b| {
                t.saw(a.get(), b.get());
                a.get().cmp(&b.get())
            });
            println!("unstable cell {len} {kind} {} {:x}", t.calls, t.hash);
            let mut t = Trace::new();
            let mut dropped: Vec<Dropped> = base.iter().map(|&x| Dropped(x)).collect();
            dropped.sort_by(|a, b| {
                t.saw(a.0, b.0);
                a.0.cmp(&b.0)
            });
            drop(dropped);
            println!(
                "stable dropped {len} {kind} {} {:x} {}",
                t.calls,
                t.hash,
                DROPS.with(|drops| drops.replace(0))
            );
        }
    }
}
