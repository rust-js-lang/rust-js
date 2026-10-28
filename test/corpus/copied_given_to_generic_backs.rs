// A copy given to generic code as its adapter type, `Copied<I>`, and
// stepped from its back there: made knowing its back where its caller makes
// it, as a filter or a map so given is (ADR 1001, #385).
fn copied_back<'a, I>(mut it: std::iter::Copied<I>) -> (Option<u32>, Option<u32>)
where
    I: DoubleEndedIterator<Item = &'a u32>,
{
    (it.next_back(), it.next())
}

fn passed_on<'a, I>(it: std::iter::Copied<I>) -> (Option<u32>, Option<u32>)
where
    I: DoubleEndedIterator<Item = &'a u32>,
{
    copied_back(it)
}

fn main() {
    let v = vec![1u32, 2, 3, 4];
    println!("{:?}", copied_back(v.iter().copied()));
    println!("{:?}", passed_on(v.iter().copied()));
    let evens: Vec<u32> = (1..9).filter(|x| x % 2 == 0).collect();
    println!("{:?}", copied_back(evens.iter().copied()));
    let picked = [7u32, 8, 9];
    println!("{:?}", copied_back(picked.iter().filter(|x| **x != 8).copied()));
}
