// The hint of an empty `cycle()` generic code asks is `(0, Some(0))`, as
// std's `Cycle::size_hint` says of one whose source gives nothing (#1979).
fn hint_of<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn main() {
    println!("{:?}", hint_of(Vec::<u32>::new().into_iter().cycle()));
    println!("{:?}", hint_of(Vec::<u32>::new().into_iter().cycle().take(3)));
    let v: Vec<u32> = Vec::new();
    println!("{:?}", hint_of(v.iter().map(|x| x + 1).cycle()));
}
