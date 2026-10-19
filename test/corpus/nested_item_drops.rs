// A drop given for an associated type of one a bound names, `<I::Item as
// IntoIterator>::Item` of an `I::Item: IntoIterator`, in turn, as an
// associated type of a type parameter's is (ADR 0478); and `Extend`'s
// `&mut Self` of one only its caller knows, given in its dictionary's box.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn firsts<I>(iterable: I) -> Vec<<I::Item as IntoIterator>::Item>
where
    I: IntoIterator,
    I::Item: IntoIterator,
{
    let mut out = Vec::new();
    for inner in iterable {
        let mut it = inner.into_iter();
        if let Some(first) = it.next() {
            out.push(first);
        }
    }
    out
}

fn concat<I>(iter: I) -> Option<I::Item>
where
    I: IntoIterator,
    I::Item: Extend<<<I as IntoIterator>::Item as IntoIterator>::Item> + IntoIterator,
{
    iter.into_iter().reduce(|mut a, b| {
        a.extend(b);
        a
    })
}

fn main() {
    let groups = vec![vec![Loud(1), Loud(2)], vec![], vec![Loud(3)]];
    let got = firsts(groups);
    println!("got {}", got.len());
    drop(got);
    let all = concat(vec![vec![Loud(10)], vec![Loud(11), Loud(12)]]);
    println!("all {}", all.as_ref().map_or(0, |v| v.len()));
    println!("end");
}
