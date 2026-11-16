// An adapter made in place, given to code that gives it back, `pass(a)`,
// a clone of it, `dup(&a)` or `a.clone()`, or a `&mut` of it,
// `pass(i.by_ref())`, then stepped from its back: made knowing its back
// where it's made, as the walk can't follow it through what it's given to
// (#1981). `filter_map`, `copied` and `cloned` of a `Vec` and of a
// string's `chars()`, each as native Rust steps it; and given to code bound
// to step it from its back, `last(a)`, `last(m.by_ref())`, or boxed a `dyn
// DoubleEndedIterator`.
fn pass<I>(it: I) -> I {
    it
}

fn dup<I: Clone>(it: &I) -> I {
    it.clone()
}

// And code that steps it from its back itself, bound so, which its type's
// question answers (#388).
fn last<I: DoubleEndedIterator>(mut it: I) -> Option<I::Item> {
    it.next_back()
}

fn main() {
    let v = vec![1, 2, 3, 4, 5, 6, 7];
    let mut a = v.clone().into_iter().filter_map(|x| if x % 2 == 1 { Some(x) } else { None });
    a.next();
    let mut b = pass(a);
    println!("filter_map pass {:?} {:?}", b.next_back(), b.nth_back(0));
    let mut c = v.iter().copied();
    c.next();
    let mut d = pass(c);
    println!("copied pass {:?} {:?}", d.next_back(), d.nth_back(1));
    let mut e = v.iter().cloned();
    let f = e.clone();
    println!("cloned clone {:?} {:?}", e.next_back(), f.rev().next());
    let mut g = v.iter().copied();
    let h = dup(&g);
    println!("copied dup {:?} {}", g.next_back(), h.count());
    let s = "abc";
    let mut i = s.chars().filter_map(|c| if c != 'b' { Some(c) } else { None });
    let mut j = pass(i.by_ref());
    println!("chars filter_map {:?}", j.next_back());
    let k = v.iter().filter_map(|x| if *x > 2 { Some(x * 10) } else { None });
    let total = pass(k).rfold(0, |acc, x| acc * 2 + x);
    println!("filter_map rfold {total}");
    let l = v.clone().into_iter().filter_map(|x| if x % 2 == 1 { Some(x) } else { None });
    println!("last filter_map {:?}", last(l));
    println!("last map {:?}", last(v.iter().map(|x| x * 2)));
    println!("last filter {:?}", last(v.iter().filter(|x| **x > 3)));
    let mut m = v.iter().filter_map(|x| if *x > 2 { Some(*x) } else { None });
    println!("last by_ref {:?} {:?}", last(m.by_ref()), m.next());
    let mut boxed: Box<dyn DoubleEndedIterator<Item = i32>> = Box::new(v.iter().map(|x| x + 100));
    println!("box {:?} {:?}", boxed.next_back(), boxed.nth_back(1));
    // A box is made knowing its back, and so its hint, a filter's of a
    // string's `chars()` as std's (#1563's, refused before).
    let s = "h\u{e9}llo w\u{f6}rld";
    let hinted: Box<dyn Iterator<Item = u32> + '_> = Box::new(s.chars().filter(|c| c.is_alphabetic()).map(|c| c as u32));
    println!("box hint {:?}", hinted.size_hint());
}
