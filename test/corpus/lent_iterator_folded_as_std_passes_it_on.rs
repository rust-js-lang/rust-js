// A `take()` of a `vec::IntoIter` lent, `a.by_ref()` or `&mut a` given on,
// folded from its back: std's `&mut I` folds a sized `I` by its
// `try_rfold`, so `Take`'s drops what it skips from its back first, 5 then
// 4 (#1986). A `&mut dyn DoubleEndedIterator` is unsized, so it folds by
// std's default `rfold` and `fold`, by its `next_back` and `next`: `Take`'s
// `next_back`, `nth_back(len - n)`, drops what it skips as one slice, 4
// then 5 (#1991); a `Box<dyn ..>`'s too. Its `try_fold` and `try_rfold`,
// called as `&mut dyn`'s, the same. Each row in a function of its own, its
// items printed as they drop.

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    (1..=5).map(D).collect()
}

fn pass<I>(it: I) -> I {
    it
}

fn lent_rfold() {
    let mut a = ds().into_iter().take(3);
    println!("a next {:?}", a.next().map(|x| x.0));
    let b = pass(a.by_ref());
    println!("b rfold {}", b.rfold(0, |acc, x| acc * 10 + x.0));
    println!("a next_back {:?}", a.next_back().map(|x| x.0));
    for x in a {
        println!("a rest {}", x.0);
    }
}

fn lent_fold() {
    let mut a = ds().into_iter().take(3);
    println!("a next_back {:?}", a.next_back().map(|x| x.0));
    let b = pass(a.by_ref());
    println!("b fold {}", b.fold(0, |acc, x| acc * 10 + x.0));
    println!("a next {:?}", a.next().map(|x| x.0));
}

fn dyn_rfold() {
    let mut a = ds().into_iter().take(3);
    println!("a next {:?}", a.next().map(|x| x.0));
    {
        let b: &mut dyn DoubleEndedIterator<Item = D> = &mut a;
        println!("b rfold {}", b.rfold(0, |acc, x| acc * 10 + x.0));
    }
    println!("a next_back {:?}", a.next_back().map(|x| x.0));
    for x in a {
        println!("a rest {}", x.0);
    }
}

fn dyn_fold() {
    let mut a = ds().into_iter().take(3);
    println!("a next_back {:?}", a.next_back().map(|x| x.0));
    {
        let b: &mut dyn DoubleEndedIterator<Item = D> = &mut a;
        println!("b fold {}", b.fold(0, |acc, x| acc * 10 + x.0));
    }
    println!("a next {:?}", a.next().map(|x| x.0));
}

fn dyn_rfold_of_a_skip() {
    let mut a = ds().into_iter().skip(1);
    {
        let b: &mut dyn DoubleEndedIterator<Item = D> = &mut a;
        println!("b rfold {}", b.rfold(0, |acc, x| acc * 10 + x.0));
    }
    println!("a next {:?}", a.next().map(|x| x.0));
}

fn boxed_dyn_rfold() {
    let mut a = ds().into_iter().take(3);
    println!("a next {:?}", a.next().map(|x| x.0));
    let b: Box<dyn DoubleEndedIterator<Item = D>> = Box::new(a);
    println!("b rfold {}", b.rfold(0, |acc, x| acc * 10 + x.0));
}

fn dyn_try_fold_of_a_skip() {
    let mut a = ds().into_iter().skip(1).take(3);
    {
        let mut b: &mut dyn DoubleEndedIterator<Item = D> = &mut a;
        let r: Option<u32> = Iterator::try_fold(&mut b, 0, |acc, x| if x.0 < 4 { Some(acc * 10 + x.0) } else { None });
        println!("b try_fold {:?}", r);
        let r: Option<u32> = DoubleEndedIterator::try_rfold(&mut b, 0, |acc, x| Some(acc * 10 + x.0));
        println!("b try_rfold {:?}", r);
    }
    println!("a next {:?}", a.next().map(|x| x.0));
}

fn main() {
    println!("lent rfold");
    lent_rfold();
    println!("lent fold");
    lent_fold();
    println!("dyn rfold");
    dyn_rfold();
    println!("dyn fold");
    dyn_fold();
    println!("dyn rfold of a skip");
    dyn_rfold_of_a_skip();
    println!("boxed dyn rfold");
    boxed_dyn_rfold();
    println!("dyn try_fold of a skip");
    dyn_try_fold_of_a_skip();
}
