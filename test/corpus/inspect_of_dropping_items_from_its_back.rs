// An `inspect()` of a `Vec`'s items with a destructor, held in a variable
// to drop them, stepped from its back: `rfind`, `next_back` then `next`,
// a `rev()` of it and of a reversed source, each seen as it's stepped and
// dropped as std's `Inspect` gives or passes it, what's left dropped as
// it's dropped (#751).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        print!("drop {} ", self.0);
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4)]
}

fn main() {
    let found = ds()
        .into_iter()
        .inspect(|d| print!("see {} ", d.0))
        .rfind(|d| d.0 % 2 == 1);
    println!("| rfind {:?}", found.as_ref().map(|d| d.0));
    drop(found);
    let mut it = ds().into_iter().inspect(|d| print!("see {} ", d.0));
    let back = it.next_back();
    println!("| next_back {:?}", back.as_ref().map(|d| d.0));
    drop(back);
    let front = it.next();
    println!("| next {:?}", front.as_ref().map(|d| d.0));
    drop(front);
    drop(it);
    println!("| dropped");
    let reversed: Vec<u32> = ds()
        .into_iter()
        .inspect(|d| print!("see {} ", d.0))
        .rev()
        .map(|d| d.0)
        .collect();
    println!("| rev {reversed:?}");
    let found = ds()
        .into_iter()
        .rev()
        .inspect(|d| print!("see {} ", d.0))
        .rfind(|d| d.0 > 2);
    println!("| rev rfind {:?}", found.as_ref().map(|d| d.0));
    drop(found);
    let mut it = ds().into_iter().inspect(|d| print!("see {} ", d.0)).rev();
    let first = it.next().map(|d| d.0);
    println!("| rev next {first:?}");
    drop(it);
    println!("| end");
}
