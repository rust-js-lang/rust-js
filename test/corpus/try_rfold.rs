// `try_rfold` folds from the back, stopping at the first `Err` or `None`,
// the rest left where it is: of generic code's iterator and of an array's
// (ADR 0531).
fn summed_back<I: DoubleEndedIterator<Item = i32>>(it: I) -> Result<i32, String> {
    it.into_iter().try_rfold(0, |acc, x| match x {
        x if x < 0 => Err(format!("bad {x} after {acc}")),
        x => Ok(acc * 10 + x),
    })
}

fn main() {
    println!("{:?} {:?}", summed_back(vec![1, 2, 3].into_iter()), summed_back(vec![1, -2, 3].into_iter()));
    let mut it = [5, 6, 7, 8].iter();
    let stopped = it.try_rfold(0u32, |acc, &x| if x == 6 { None } else { Some(acc + x) });
    println!("{stopped:?} {:?}", it.next());
    let all: Option<u32> = [1u32, 2, 3].iter().try_rfold(1u32, |acc, &x| acc.checked_mul(x + 1));
    println!("{all:?}");
}
