// Each std adapter's `size_hint`, of the crate's iterator whose own prints
// as it's asked, and of a range: std's, asking its source where std's
// asks it and no more (ADR 1092's table, #733). And what asks it where std
// does: a collect into a `String` of `char`s, or a hash table, before it
// steps, as their `extend` reserves by it, and a `filter()`'s `count()`,
// its source's first.
use std::collections::{HashMap, HashSet};

struct Own {
    at: u32,
    end: u32,
}

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.at += 1;
        (self.at <= self.end).then_some(self.at)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("  own hint at {}", self.at);
        let left = (self.end - self.at.min(self.end)) as usize;
        (left, Some(left))
    }
}

/// One that writes its own `fold` too, which a collect folds it by.
struct Folds(Own);

impl Iterator for Folds {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
    fn fold<B, F: FnMut(B, u32) -> B>(self, init: B, f: F) -> B {
        println!("  own fold");
        let mut own = self.0;
        own.by_ref().fold(init, f)
    }
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn own() -> Own {
    Own { at: 0, end: 5 }
}

fn show(name: &str, hint: (usize, Option<usize>)) {
    println!("{name} {hint:?}");
}

// A closure made in generic code, whose type holds its type parameters,
// is no iterator a `filter()` steps.
fn in_generic<T: std::fmt::Debug>(tag: T) {
    println!("{tag:?}");
    show("filter in generic", own().filter(|x| x % 2 == 1).size_hint());
}

fn main() {
    show("filter", own().filter(|x| x % 2 == 1).size_hint());
    show("filter_map", own().filter_map(|x| (x % 2 == 1).then_some(x)).size_hint());
    show("take_while", own().take_while(|&x| x < 3).size_hint());
    let mut done = own().take_while(|&x| x < 2);
    done.by_ref().for_each(drop);
    show("take_while done", done.size_hint());
    show("skip_while", own().skip_while(|&x| x < 3).size_hint());
    show("map_while", own().map_while(|x| (x < 4).then_some(x)).size_hint());
    show("scan", own().scan(0, |s, x| { *s += x; Some(*s) }).size_hint());
    show("take 0", own().take(0).size_hint());
    show("take 3", own().take(3).size_hint());
    show("take 9", own().take(9).size_hint());
    show("skip 2", own().skip(2).size_hint());
    show("chain", own().chain(own()).size_hint());
    show("zip", own().zip(1..3u32).size_hint());
    show("enumerate", own().enumerate().size_hint());
    show("flat_map", own().flat_map(|x| vec![x, x]).size_hint());
    let mut flat = own().flat_map(|x| vec![x, x, x]);
    flat.next();
    show("flat_map in one", flat.size_hint());
    show("range filter", (1..6u32).filter(|x| x % 2 == 1).size_hint());
    // Of what drops its items, closed where it's dropped: its source's.
    let ds = vec![D(1), D(2), D(3)];
    show("rev filter", ds.into_iter().rev().filter(|d| d.0 > 1).size_hint());
    in_generic("tag");
    let s: String = own().map(|x| char::from(b'a' + x as u8)).collect();
    println!("{s}");
    let h: HashSet<u32> = own().filter(|x| x % 2 == 1).collect();
    println!("{}", h.len());
    let m: HashMap<u32, u32> = own().map(|x| (x, x * x)).collect();
    println!("{}", m.len());
    println!("{}", own().filter(|x| x % 2 == 1).count());
    let folded: String = Folds(own()).map(|x| char::from(b'a' + x as u8)).collect();
    println!("{folded}");
}
