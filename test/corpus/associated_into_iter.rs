// `into_iter()` of a type parameter's associated type, `H::Item: IntoIterator`,
// as itertools' multi_cartesian_product steps each of its iterables: what
// the caller gives, an array or a JS iterator, stepped by Iterator.from
// (ADR 0061).
fn lengths<H>(iters: H) -> Vec<usize>
where
    H: Iterator,
    H::Item: IntoIterator,
{
    iters.map(|i| i.into_iter().count()).collect()
}

fn firsts<H>(iters: H) -> Vec<Option<<H::Item as IntoIterator>::Item>>
where
    H: Iterator,
    H::Item: IntoIterator,
{
    let mut out = Vec::new();
    for i in iters {
        out.push(i.into_iter().next());
    }
    out
}

fn main() {
    let v = vec![vec![1, 2], vec![], vec![3]];
    println!("{:?} {:?}", lengths(v.clone().into_iter()), firsts(v.into_iter()));
}
