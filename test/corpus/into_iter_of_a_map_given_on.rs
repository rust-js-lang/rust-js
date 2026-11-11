// `IntoIterator::into_iter` of a std adapter held as std's struct, a
// `map()`, gives the adapter itself on, to generic code that clones it,
// as itertools' `iproduct!` gives what it's given to `cartesian_product`:
// it isn't stepped there, so it's std's struct, which its compiled `Clone`
// clones, not the recognizers' JS iterator (ADR 0871; #1409).
fn paired<I, J>(a: I, b: J) -> i32
where
    I: Iterator<Item = i32>,
    J: Iterator<Item = i32> + Clone,
{
    a.map(|x| x * b.clone().sum::<i32>()).sum()
}

fn main() {
    let total = paired(
        IntoIterator::into_iter([1, 2, 3].iter().map(|x| x + 1)),
        IntoIterator::into_iter([1, 2].iter().map(|x| x * 10)),
    );
    println!("{total}");
}
