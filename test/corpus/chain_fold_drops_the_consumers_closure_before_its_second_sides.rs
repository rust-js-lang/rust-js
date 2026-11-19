// A chain folded by value whose second side is a `map` or a `filter` with a
// closure that drops: `Chain::fold` gives `b.fold` its `f`, so the consumer's
// closure drops with the second side's fold, before a `map`'s own function, as
// `map_fold(f, g)` drops `g` first, and after a `filter`'s, and a
// `for_each`'s alike (#734).
struct D(&'static str);

impl Drop for D {
    fn drop(&mut self) {
        print!("drop {} ", self.0);
    }
}

fn zero(_d: &D) -> u32 {
    0
}

fn main() {
    let f = D("map");
    let g = D("fold");
    let total = (0..2u32)
        .chain(vec![5u32, 6].into_iter().map(move |x| x + zero(&f)))
        .fold(0, move |a, x| a + x + zero(&g));
    println!("| {total}");
    let f = D("map2");
    let g = D("fold2");
    let total = [1u32, 2]
        .into_iter()
        .chain([3u32].into_iter().map(move |x| x + zero(&f)))
        .fold(0, move |a, x| a + x + zero(&g));
    println!("| {total}");
    let f = D("filter");
    let g = D("fold3");
    let total = (0..2u32)
        .chain((5..9u32).filter(move |x| x % 2 == zero(&f)))
        .fold(0, move |a, x| a + x + zero(&g));
    println!("| {total}");
    let f = D("map4");
    let g = D("each");
    (0..2u32)
        .chain(vec![5u32].into_iter().map(move |x| x + zero(&f)))
        .for_each(move |x| print!("{} ", x + zero(&g)));
    println!("|");
}
