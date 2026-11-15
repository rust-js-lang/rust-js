// A held flat map's adapters, `skip`, `map` and `take`, stepped once, then
// collected in a crate that reads capacities: each is std's struct, whose
// hint is its flat map's front and back inners' (#1528), where JS's own
// `flatMap` has none: capacities 42, 46 and 46, where the hand form gives
// 64, 64 and 32. A `rev()` of one, whose hand form keeps the hint (#1892),
// too.
fn main() {
    let v = [1u32, 2, 3];
    let mut skipped = v.iter().flat_map(|&x| vec![x; 12]).skip(2);
    skipped.next();
    let rest: Vec<u32> = skipped.collect();
    println!("{} {}", rest.len(), rest.capacity());

    let mut mapped = v.iter().flat_map(|&x| vec![x; 12]).map(|x| x + 1);
    mapped.next();
    let rest: Vec<u32> = mapped.collect();
    println!("{} {}", rest.len(), rest.capacity());

    let mut taken = v.iter().flat_map(|&x| vec![x; 12]).take(30);
    taken.next();
    let rest: Vec<u32> = taken.collect();
    println!("{} {}", rest.len(), rest.capacity());

    let mut back = v.iter().flat_map(|&x| vec![x; 12]).rev();
    back.next();
    let rest: Vec<u32> = back.collect();
    println!("{} {}", rest.len(), rest.capacity());
}
