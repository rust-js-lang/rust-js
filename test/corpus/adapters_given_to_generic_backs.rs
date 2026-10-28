// A filter or a map given to generic code as its adapter type, `Filter<I,
// P>`, and stepped from its back there, or by what that code gives it to:
// made knowing its back where its caller makes it, as what the generic
// code steps is carried to each caller's types (#388).
fn back<I, P>(mut it: std::iter::Filter<I, P>) -> (Option<u32>, Option<u32>)
where
    I: DoubleEndedIterator<Item = u32>,
    P: FnMut(&u32) -> bool,
{
    (it.next_back(), it.next())
}

fn passed_on<I, P>(it: std::iter::Filter<I, P>) -> (Option<u32>, Option<u32>)
where
    I: DoubleEndedIterator<Item = u32>,
    P: FnMut(&u32) -> bool,
{
    back(it)
}

fn last_mapped<I, F>(mut it: std::iter::Map<I, F>) -> Option<u32>
where
    I: DoubleEndedIterator<Item = u32>,
    F: FnMut(u32) -> u32,
{
    it.next_back()
}

fn main() {
    let v = vec![1u32, 2, 3, 4, 5];
    println!("{:?}", back(v.iter().copied().filter(|x| x % 2 == 1)));
    println!("{:?}", passed_on(v.iter().copied().filter(|x| *x > 2)));
    println!("{:?}", last_mapped((1..4u32).map(|x| x * 10)));
}
