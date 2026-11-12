// `collect()` of `Option`s or `Result`s into the crate's own
// `FromIterator` calls its `from_iter` with what stops at the first `None`
// or `Err`, as std's `try_process` does, then drops what it made where one
// stopped it (#1588): its effects interleave with the items' steps, what's
// left of the items drops before what it made, and an `Err` it stopped at
// drops as a panic in `from_iter` unwinds; what it's given is its own,
// which it may forget, and stepped past a `None` or an `Err`, it goes on,
// a later `Err` kept in place of the one before. So of generic code's `C`,
// and of generic code's `B: FromIterator` given an `Option<Bag>`, whose
// dictionary drops what's given as generic code gives its drop.
use std::panic::catch_unwind;

#[derive(Debug)]
struct Bag(Vec<i32>);

impl FromIterator<i32> for Bag {
    fn from_iter<I: IntoIterator<Item = i32>>(it: I) -> Self {
        println!("from_iter");
        let mut held = Vec::new();
        for x in it {
            println!("bag takes {x}");
            held.push(x);
        }
        println!("bag made");
        Bag(held)
    }
}

impl Drop for Bag {
    fn drop(&mut self) {
        println!("drop Bag {:?}", self.0);
    }
}

#[derive(Debug)]
struct Loud(i32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop Loud {}", self.0);
    }
}

struct Picky;

impl FromIterator<i32> for Picky {
    fn from_iter<I: IntoIterator<Item = i32>>(it: I) -> Self {
        let n = it.into_iter().count();
        println!("picky counted {n}");
        panic!("picky");
    }
}

struct First(i32);

impl FromIterator<i32> for First {
    fn from_iter<I: IntoIterator<Item = i32>>(it: I) -> Self {
        let mut it = it.into_iter();
        let first = it.next().unwrap_or(0);
        std::mem::forget(it);
        First(first)
    }
}

struct Past(Vec<i32>);

impl FromIterator<i32> for Past {
    fn from_iter<I: IntoIterator<Item = i32>>(it: I) -> Self {
        let mut it = it.into_iter();
        let mut got = Vec::new();
        for _ in 0..4 {
            let x = it.next();
            println!("past got {x:?}");
            got.extend(x);
        }
        Past(got)
    }
}

fn collect_as<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

fn gather<C: FromIterator<i32>>(items: Vec<Option<i32>>) -> Option<C> {
    items.into_iter().collect()
}

fn main() {
    let none: Option<Bag> = [Some(5), None, Some(6)]
        .into_iter()
        .inspect(|x| println!("step {x:?}"))
        .collect();
    println!("{none:?}");
    let first: Option<Bag> = [None, Some(1)].into_iter().collect();
    println!("{first:?}");
    let last: Option<Bag> = [Some(1), Some(2), None].into_iter().collect();
    println!("{last:?}");
    let err_first: Result<Bag, u8> = [Err(1), Ok(2)].into_iter().collect();
    println!("{err_first:?}");
    let err_last: Result<Bag, u8> = [Ok(3), Ok(4), Err(2)].into_iter().collect();
    println!("{err_last:?}");
    let some: Option<Bag> = [Some(1), Some(2)].into_iter().collect();
    println!("{some:?}");
    let err: Result<Bag, String> = [Ok(3), Err("no".to_string()), Ok(4)].into_iter().collect();
    println!("{err:?}");
    let ok: Result<Bag, String> = vec![Ok(7)].into_iter().collect();
    println!("{ok:?}");
    let left: Option<Bag> = vec![(Some(8), Loud(1)), (None, Loud(2)), (Some(9), Loud(3))]
        .into_iter()
        .map(|(x, _l)| x)
        .collect();
    println!("{left:?}");
    let through: Option<Bag> = collect_as(
        vec![(Some(13), Loud(9)), (None, Loud(10)), (Some(14), Loud(11))]
            .into_iter()
            .map(|(x, _l)| x),
    );
    println!("{through:?}");
    let kept: Option<First> = vec![(Some(11), Loud(5)), (Some(12), Loud(6))]
        .into_iter()
        .map(|(x, _l)| x)
        .collect();
    println!("first {:?}", kept.map(|f| f.0));
    let generic: Option<Bag> = gather(vec![Some(10), None]);
    println!("{generic:?}");
    let o: Option<Past> = [Some(1), None, Some(2), None].into_iter().collect();
    println!("{:?}", o.map(|p| p.0));
    let r: Result<Past, Loud> = vec![Ok(1), Err(Loud(7)), Ok(2), Err(Loud(8))].into_iter().collect();
    println!("{:?}", r.map(|p| p.0));
    let caught = catch_unwind(|| {
        let r: Result<Picky, Loud> = vec![Ok(1), Err(Loud(4)), Ok(2)].into_iter().collect();
        r.is_ok()
    });
    println!("caught {}", caught.is_err());
}
