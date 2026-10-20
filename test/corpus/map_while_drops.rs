// `map_while(f)` is `map(f)`, then its `Some`s till the first `None`, as
// `take_while` takes them, each an item's `f` owns, of what has a
// destructor too; and what `filter_map` and `map_while` keep that looks
// like `None`, `Some(None)`'s, is taken out of its box, where `find_map`'s
// `Option` keeps it (ADR 0515).
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn mapped() {
    let v = vec![1, 2, -3, 4];
    let pos: Vec<u32> = v.iter().map_while(|&x| u32::try_from(x).ok()).collect();
    println!("{pos:?}");
    let nested: Vec<Option<u8>> = vec![Some(Some(1)), Some(None), None, Some(Some(2))]
        .into_iter()
        .map_while(|x| x)
        .collect();
    println!("{nested:?}");
    let words: Vec<&str> = "a b  c"
        .split(' ')
        .map_while(|w| (!w.is_empty()).then_some(w))
        .collect();
    println!("{words:?}");
    let louds = vec![Loud(1), Loud(2), Loud(0), Loud(3)];
    let kept: Vec<u8> = louds.into_iter().map_while(|l| (l.0 > 0).then(|| l.0)).collect();
    println!("{kept:?}");
}
fn kept() {
    let flat: Vec<Option<u8>> = vec![Some(Some(1)), Some(None), None]
        .into_iter()
        .flat_map(|x| x)
        .collect();
    println!("{flat:?}");
    let nested: Vec<Option<u8>> = vec![Some(Some(1)), Some(None), None, Some(Some(2))]
        .into_iter()
        .filter_map(|x| x)
        .collect();
    println!("{nested:?}");
    let found = vec![Some(Some(1)), Some(None)]
        .into_iter()
        .find_map(|x| x.filter(|y| y.is_none()));
    println!("{found:?}");
}
fn main() {
    mapped();
    kept();
    println!("end");
}
