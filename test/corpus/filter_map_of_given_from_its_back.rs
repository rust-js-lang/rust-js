//@ library-differs
// A `filter_map` of an iterator a struct holds as it is, folded from its
// back, as itertools' `FilterMapOk::rfold` folds `self.iter.filter_map(..)`:
// made knowing its back, by its source's `DoubleEndedIterator` dictionary
// (ADR 0488, #473).

struct Oks<I> {
    iter: I,
}

impl<I: Iterator<Item = Result<u32, String>>> Iterator for Oks<I> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.iter.by_ref().find_map(Result::ok)
    }
}

impl<I: DoubleEndedIterator<Item = Result<u32, String>>> DoubleEndedIterator for Oks<I> {
    fn next_back(&mut self) -> Option<u32> {
        self.iter.by_ref().rev().find_map(Result::ok)
    }

    fn rfold<Acc, G: FnMut(Acc, u32) -> Acc>(self, init: Acc, f: G) -> Acc {
        self.iter.filter_map(|result| result.ok().map(|n| n * 2)).rfold(init, f)
    }
}

/// One whose kept values look like `None`, each `Some` boxed.
struct Somes<I> {
    iter: I,
}

impl<I: Iterator<Item = Result<Option<u32>, String>>> Iterator for Somes<I> {
    type Item = Option<u32>;

    fn next(&mut self) -> Option<Option<u32>> {
        self.iter.by_ref().find_map(Result::ok)
    }
}

impl<I: DoubleEndedIterator<Item = Result<Option<u32>, String>>> DoubleEndedIterator for Somes<I> {
    fn next_back(&mut self) -> Option<Option<u32>> {
        self.iter.by_ref().rev().find_map(Result::ok)
    }

    fn rfold<Acc, G: FnMut(Acc, Option<u32>) -> Acc>(self, init: Acc, f: G) -> Acc {
        self.iter.filter_map(Result::ok).rfold(init, f)
    }
}

fn main() {
    let somes = Somes { iter: vec![Ok(Some(1)), Err("no".to_string()), Ok(None), Ok(Some(4))].into_iter() };
    println!("{:?}", somes.rev().collect::<Vec<_>>());
    let somes = Somes { iter: vec![Ok(None), Ok(Some(2)), Err("no".to_string())].into_iter() };
    let folded = somes.rfold(String::new(), |shown, kept| format!("{shown}[{kept:?}]"));
    println!("{folded}");
    let results = vec![Ok(1), Err("no".to_string()), Ok(3), Ok(4), Err("not".to_string())];
    let oks = Oks { iter: results.clone().into_iter() };
    println!("{:?}", oks.rev().collect::<Vec<_>>());
    let mut oks = Oks { iter: results.into_iter() };
    println!("{:?} {:?}", oks.next_back(), oks.next());
    let doubled = Oks { iter: vec![Ok(5), Ok(6)].into_iter() }.rfold(0, |sum, n| sum * 100 + n);
    println!("{doubled}");
}
