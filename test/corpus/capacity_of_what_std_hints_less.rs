// A `Vec` collected, in a crate that reads a capacity, of an iterator that
// isn't `TrustedLen` whose items JS counts as an array: std grows it by its
// hint's lower bound, which a B-tree's `range(..)`, a set's
// `intersection(..)` and `symmetric_difference(..)` and a hash set's
// `difference(..)` give as 0, where JS's array counted what was left (#1215);
// and by its length of what's exact-sized, a hash set's or a heap's iterator
// enumerated, a map's `values_mut()`, drains and `extract_if(..)`.
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};

fn ranges(n: u32) {
    let b: BTreeSet<u32> = (0..n).collect();
    let c: BTreeSet<u32> = (n / 2..n / 2 + n / 4).collect();
    let h: HashSet<u32> = (0..n).collect();
    let g: HashSet<u32> = (n / 2..n / 2 + n / 4).collect();
    let mut m: BTreeMap<u32, u32> = (0..n).map(|i| (i, i)).collect();
    let none = c.iter().filter(|x| **x > 100);
    let r: Vec<&u32> = b.range(2..).collect();
    let rm = m.range(2..).collect::<Vec<(&u32, &u32)>>().capacity();
    let rr: Vec<&u32> = b.range(2..).rev().collect();
    let rc: Vec<&u32> = b.range(2..).chain(none).collect();
    let rp: Vec<&u32> = b.range(2..).peekable().collect();
    let rs: Vec<&u32> = b.range(2..).skip(1).collect();
    let rk = b.range(2..);
    let rk2 = rk.clone();
    let rkv: Vec<&u32> = rk.collect();
    let rkc: Vec<&u32> = rk2.skip(3).collect();
    let i: Vec<&u32> = b.intersection(&c).collect();
    let s: Vec<&u32> = b.symmetric_difference(&c).collect();
    let hd: Vec<&u32> = h.difference(&g).collect();
    let hi: Vec<&u32> = h.intersection(&g).collect();
    let hs: Vec<&u32> = h.symmetric_difference(&g).collect();
    let mutated = m.range_mut(5..).map(|(_, v)| *v).collect::<Vec<u32>>().capacity();
    let copied: Vec<u32> = b.range(2..).copied().collect();
    let cloned: Vec<u32> = b.range(2..).rev().cloned().collect();
    let none = c.iter().filter(|x| **x > 100);
    let rev_chained: Vec<&u32> = b.range(2..).rev().chain(none).collect();
    println!(
        "{n}: {} {rm} {} {} {} {} {} {} {} {} {} {} {} {mutated} {} {} {}",
        r.capacity(),
        rr.capacity(),
        rc.capacity(),
        rp.capacity(),
        rs.capacity(),
        rkv.capacity(),
        rkc.capacity(),
        i.capacity(),
        s.capacity(),
        hd.capacity(),
        hi.capacity(),
        hs.capacity(),
        copied.capacity(),
        cloned.capacity(),
        rev_chained.capacity()
    );
}

fn exact(n: u32) {
    let mut hm: HashMap<u32, u32> = (0..n).map(|i| (i, i)).collect();
    let mut bm: BTreeMap<u32, u32> = (0..n).map(|i| (i, i)).collect();
    let mut hs: HashSet<u32> = (0..n).collect();
    let mut bs: BTreeSet<u32> = (0..n).collect();
    let heap: BinaryHeap<u32> = (0..n).collect();
    let mut dq: VecDeque<u32> = (0..n).collect();
    let enumerated = hs.iter().enumerate().collect::<Vec<(usize, &u32)>>().capacity();
    let heap_enumerated: Vec<(usize, &u32)> = heap.iter().enumerate().collect();
    let heap_skipped: Vec<&u32> = heap.iter().skip(2).collect();
    let copied = hs
        .iter()
        .copied()
        .enumerate()
        .skip(1)
        .collect::<Vec<(usize, u32)>>()
        .capacity();
    let zipped = hs.iter().zip(heap.iter()).collect::<Vec<(&u32, &u32)>>().capacity();
    let hp: Vec<&u32> = heap.iter().collect();
    let dr = dq.range(dq.len().min(2)..).collect::<Vec<&u32>>().capacity();
    let he: Vec<(u32, u32)> = hm.extract_if(|k, _| *k % 2 == 0).collect();
    let be: Vec<(u32, u32)> = bm.extract_if(.., |k, _| *k % 2 == 0).collect();
    let hse: Vec<u32> = hs.extract_if(|k| *k % 2 == 0).collect();
    let bse: Vec<u32> = bs.extract_if(.., |k| *k % 2 == 0).collect();
    let hv = hm.values_mut().collect::<Vec<&mut u32>>().capacity();
    let hvm = hm.values_mut().map(|x| *x + 1).collect::<Vec<u32>>().capacity();
    let bk: Vec<u32> = bm.clone().into_keys().collect();
    let hd: Vec<(u32, u32)> = hm.drain().collect();
    let hsd: Vec<u32> = hs.drain().collect();
    let dd: Vec<u32> = dq.drain(dq.len().min(3)..).collect();
    println!(
        "{n}: {enumerated} {} {} {copied} {zipped} {} {dr} {} {} {} {} {hv} {hvm} {} {} {} {}",
        heap_enumerated.capacity(),
        heap_skipped.capacity(),
        hp.capacity(),
        he.capacity(),
        be.capacity(),
        hse.capacity(),
        bse.capacity(),
        bk.capacity(),
        hd.capacity(),
        hsd.capacity(),
        dd.capacity()
    );
}

fn main() {
    for n in [0, 1, 3, 5, 9, 40] {
        ranges(n);
        exact(n);
    }
}
