// `rev()`s, `enumerate()`s and `map()`s a macro's repetition writes at one
// span, one given where it's stepped as a JS iterator, a generic function's
// `I`, the other to a function that holds it as std's struct and steps it
// from its back: of two apart, both are std's struct, never one given the
// other's in-place form (#1791; #1671 change 3 M5). And what's made in
// place for one of them, a variable it's made of, `rev()`'s `ma`, or what
// the macro is given, `a.iter().rev()` given its `enumerate()`, asks it as
// the lowering makes it, of both.
use std::iter::{Enumerate, Map, Rev};
use std::slice::Iter;

macro_rules! each_rev {
    ($($f:ident($v:expr)),*) => {
        vec![$($f($v.iter().rev())),*]
    };
}

macro_rules! each_enumerate {
    ($($f:ident($v:expr)),*) => {
        vec![$($f($v.iter().enumerate())),*]
    };
}

macro_rules! each_rev_of {
    ($($f:ident($v:ident)),*) => {
        vec![$($f($v.rev())),*]
    };
}

macro_rules! each_enumerated {
    ($($f:ident($e:expr)),*) => {
        vec![$($f($e.enumerate())),*]
    };
}

macro_rules! each_map {
    ($($f:ident($v:expr)),*) => {
        vec![$($f($v.iter().map(|x: &u32| *x * 10))),*]
    };
}

fn stepped<I: Iterator<Item = u32>>(it: I) -> Vec<u32> {
    it.collect()
}

fn first_of<'a, I: Iterator<Item = &'a u32>>(it: I) -> Vec<u32> {
    it.copied().take(2).collect()
}

fn held(mut it: Rev<Iter<'_, u32>>) -> Vec<u32> {
    let last = it.next_back().copied();
    let mut out: Vec<u32> = it.copied().collect();
    out.extend(last);
    out
}

fn numbered<'a, I: Iterator<Item = (usize, &'a u32)>>(it: I) -> Vec<u32> {
    it.map(|(i, x)| i as u32 * 100 + x).collect()
}

fn held_numbered(mut it: Enumerate<Iter<'_, u32>>) -> Vec<u32> {
    let last = it.next_back().map(|(i, x)| i as u32 * 1000 + x);
    let mut out: Vec<u32> = it.map(|(i, x)| i as u32 * 100 + x).collect();
    out.extend(last);
    out
}

fn held_mapped<'a, F: FnMut(&'a u32) -> u32>(mut it: Map<Iter<'a, u32>, F>) -> Vec<u32> {
    let last = it.next_back();
    let mut out: Vec<u32> = it.collect();
    out.extend(last);
    out.push(0);
    out
}

fn held_rev<'a, F: FnMut(&'a u32) -> u32>(mut it: Rev<Map<Iter<'a, u32>, F>>) -> Vec<u32> {
    let first = it.next_back();
    let mut out: Vec<u32> = it.collect();
    out.extend(first);
    out.push(0);
    out
}

fn held_numbered_rev(mut it: Enumerate<Rev<Iter<'_, u32>>>) -> Vec<u32> {
    let last = it.next_back().map(|(i, x)| i as u32 * 1000 + x);
    let mut out: Vec<u32> = it.map(|(i, x)| i as u32 * 100 + x).collect();
    out.extend(last);
    out
}

fn main() {
    let a = vec![1u32, 2, 3];
    let b = vec![4u32, 5, 6, 7];
    println!("{:?}", each_rev!(first_of(a), held(b)));
    println!("{:?}", each_rev!(held(a), first_of(b)));
    println!("{:?}", each_enumerate!(numbered(a), held_numbered(b)));
    println!("{:?}", each_enumerate!(held_numbered(a), numbered(b)));
    println!("{:?}", each_map!(stepped(a), held_mapped(b)));
    println!("{:?}", each_map!(held_mapped(a), stepped(b)));
    let ma = a.iter().map(|x: &u32| *x + 1);
    let mb = b.iter().map(|x: &u32| *x + 1);
    println!("{:?}", each_rev_of!(stepped(ma), held_rev(mb)));
    let ma = a.iter().map(|x: &u32| *x * 2);
    let mb = b.iter().map(|x: &u32| *x * 2);
    println!("{:?}", each_rev_of!(held_rev(ma), stepped(mb)));
    println!(
        "{:?}",
        each_enumerated!(numbered(a.iter().rev()), held_numbered_rev(b.iter().rev()))
    );
    println!(
        "{:?}",
        each_enumerated!(held_numbered_rev(a.iter().rev()), numbered(b.iter().rev()))
    );
    // Of two alike, each is the recognizers' form, as one alone is.
    println!("{:?}", each_rev!(first_of(a), first_of(b)));
    println!("{:?}", each_enumerate!(numbered(a), numbered(b)));
    println!("{:?}", each_map!(stepped(a), stepped(b)));
}
