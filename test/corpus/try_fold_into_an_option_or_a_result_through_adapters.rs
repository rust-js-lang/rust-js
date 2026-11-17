// `try_fold` and `try_rfold` of std's adapters, a `rev()`'s and a `map`'s,
// into an `Option` or a `Result`: with std's iterator structs, their
// compiled bodies are given the `Option`'s or the `Result`'s `Try` (ADR
// 0913, #1864). `Some` and a `None` that stops it; an empty one, its
// `init` given back as its output; `Some(None)`, an
// `Option` of an `Option`; an `Option<()>`; an `Ok` and an `Err` of a type
// with a destructor, the rest left unstepped; a closure whose `?` turns
// one error type into another by `From`, in its own code; and a
// `ControlFlow`'s, as before.
use std::ops::ControlFlow;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop Loud {}", self.0);
    }
}

struct Small(u32);

struct Big(u64);

impl From<Small> for Big {
    fn from(small: Small) -> Big {
        Big(u64::from(small.0) * 1000)
    }
}

fn checked(x: u32) -> Result<u32, Small> {
    if x < 5 { Ok(x) } else { Err(Small(x)) }
}

fn summed(v: &[u32]) -> Option<u32> {
    v.iter().map(|x| x * 2).rev().try_fold(0u32, |a, x| a.checked_add(x))
}

fn summed_back(v: &[u32]) -> Option<u32> {
    v.iter().map(|x| x + 1).try_rfold(0u32, |a, x| a.checked_add(x))
}

fn last_nonzero(v: &[u32]) -> Option<Option<u32>> {
    v.iter().map(|x| *x).try_rfold(None, |last, x| {
        if x == 99 {
            None
        } else {
            Some(if x == 0 { last } else { Some(x) })
        }
    })
}

fn all_small(v: &[u32]) -> Option<()> {
    v.iter().map(|x| x + 0).try_rfold((), |(), x| (x < 5).then_some(()))
}

fn until_loud(v: &[u32]) -> Result<u32, Loud> {
    v.iter()
        .map(|x| x * 1)
        .try_rfold(0u32, |a, x| if x > 3 { Err(Loud(x)) } else { Ok(a + x) })
}

fn converted(v: &[u32]) -> Result<u32, Big> {
    v.iter().map(|x| x + 0).try_rfold(0u32, |a, x| {
        let small = checked(x)?;
        Ok(a + small)
    })
}

fn flow(v: &[u32]) -> ControlFlow<u32, u32> {
    v.iter().map(|x| x * 3).try_fold(0u32, |a, x| {
        if x > 6 {
            ControlFlow::Break(x)
        } else {
            ControlFlow::Continue(a + x)
        }
    })
}

fn main() {
    println!("{:?} {:?}", summed(&[1, 2, 3]), summed(&[u32::MAX / 2, 1]));
    println!("{:?} {:?}", summed_back(&[1, 2, 3]), summed_back(&[u32::MAX - 1, 1]));
    println!("{:?} {:?}", last_nonzero(&[3, 0, 4, 0]), last_nonzero(&[]));
    println!("{:?} {:?}", last_nonzero(&[0, 0]), last_nonzero(&[1, 99, 2]));
    println!("{:?} {:?}", all_small(&[1, 2]), all_small(&[1, 7, 2]));
    match until_loud(&[1, 2, 3]) {
        Ok(n) => println!("ok {n}"),
        Err(l) => println!("err {}", l.0),
    }
    match until_loud(&[1, 5, 2, 9]) {
        Ok(n) => println!("ok {n}"),
        Err(l) => println!("err {}", l.0),
    }
    let shown = |r: Result<u32, Big>| r.map_err(|big| big.0);
    println!(
        "{:?} {:?}",
        shown(converted(&[1, 2, 3])),
        shown(converted(&[1, 7, 2, 9]))
    );
    println!("{:?} {:?}", shown(converted(&[])), summed_back(&[]));
    println!("{:?} {:?}", flow(&[1, 2]), flow(&[1, 2, 3, 4]));
}
