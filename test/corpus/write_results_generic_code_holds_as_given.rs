//@ library-differs
// Write results, `undefined` or their error in JS (ADRs 0187, 0462), where
// generic code holds them as they're given, beside the shapes refused until
// a crossing converts them (#2559, #2560): an impl whose method gives one
// used where its type is known, and another impl of its trait given to
// generic code; a `Result<(), Self::E>` entry, which gives its result
// tagged (`entry_writes_tagged`, #2057); items generic code holds as a type
// parameter's, counted and shown, std's own collect of them into a
// `Result` and its `from_iter` called as a function, which read them
// tagged (`std_reads_tagged`), a clone through generic code and a
// closure's result given back by it.

use std::fmt;
use std::fmt::Debug;

trait Make {
    type Out;
    fn make(&self) -> Self::Out;
}

fn made_twice<M: Make>(m: &M) -> (M::Out, M::Out) {
    (m.make(), m.make())
}

struct Bad;

impl Make for Bad {
    type Out = fmt::Result;
    fn make(&self) -> fmt::Result {
        Err(fmt::Error)
    }
}

impl Make for u8 {
    type Out = u8;
    fn make(&self) -> u8 {
        *self + 1
    }
}

trait Check {
    type E;
    fn check(&self) -> Result<(), Self::E>;
}

fn checked_twice<C: Check>(c: &C) -> (bool, bool) {
    (c.check().is_err(), c.check().is_ok())
}

impl Check for Bad {
    type E = fmt::Error;
    fn check(&self) -> fmt::Result {
        Err(fmt::Error)
    }
}

fn count<T, I: Iterator<Item = T>>(it: I) -> usize {
    it.count()
}

fn shown<I: Iterator>(it: I) -> String
where
    I::Item: Debug,
{
    it.map(|x| format!("{x:?}")).collect::<Vec<_>>().join(",")
}

fn dup<T: Clone>(t: &T) -> (T, T) {
    (t.clone(), t.clone())
}

fn twice<R, F: Fn() -> R>(f: F) -> (R, R) {
    (f(), f())
}

fn main() {
    let a = Bad.make();
    println!("{a:?} {}", a.is_err());
    println!("{:?}", made_twice(&3u8));
    println!("{:?}", checked_twice(&Bad));
    let v: Vec<fmt::Result> = vec![Ok(()), Err(fmt::Error)];
    println!("{}", count(v.clone().into_iter()));
    println!("{}", shown(v.clone().into_iter()));
    let all: Result<Vec<()>, fmt::Error> = v.clone().into_iter().collect();
    println!("{all:?}");
    let from: Result<Vec<()>, fmt::Error> = Result::from_iter(v);
    println!("{from:?}");
    let e: fmt::Result = Err(fmt::Error);
    println!("{:?}", dup(&e));
    println!("{:?}", twice(|| -> fmt::Result { Err(fmt::Error) }));
}
