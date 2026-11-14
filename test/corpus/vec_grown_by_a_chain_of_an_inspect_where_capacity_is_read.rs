// A `Vec` grown by a chain of an `inspect()` of a `Vec`'s items and an
// array's, in a crate that reads its capacity, which std makes its
// iterator's hint's lower bound, each part's summed: in the build of std's
// structs, std's compiled `Chain` of a compiled `Inspect`, whose hint is
// std's body's over its parts', exact, where it was refused (#1737).
fn main() {
    let mut seen = 0;
    let chained: Vec<u32> = vec![1u32, 2, 3]
        .into_iter()
        .inspect(|_| seen += 1)
        .chain([9, 8])
        .collect();
    println!("{chained:?} {} {seen}", chained.capacity());
}
