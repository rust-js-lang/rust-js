// A `Vec` into an array of its length, `<[T; N]>::try_from(v)` and
// `v.try_into()`, as aho-corasick's buckets are made: the `Vec` moved, its
// `Err` the `Vec` given back where its length isn't `N`.

#[derive(Clone, Debug)]
struct Mask(u8);

fn main() {
    let mut buckets = <[Vec<u32>; 3]>::try_from(vec![vec![]; 3]).unwrap();
    buckets[0].push(1);
    let masks: Vec<Mask> = (0..2).map(Mask).collect();
    let fixed = <[Mask; 2]>::try_from(masks).unwrap();
    let wrong = <[u8; 2]>::try_from(vec![1, 2, 3]);
    let ok: Result<[u8; 3], _> = vec![4, 5, 6].try_into();
    let back = match wrong {
        Ok(_) => 0,
        Err(v) => v.len(),
    };
    println!("{buckets:?} {fixed:?} {back} {ok:?} {}", fixed[1].0);
}
