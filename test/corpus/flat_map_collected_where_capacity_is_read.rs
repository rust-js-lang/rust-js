// A `flat_map` or a `flatten` a `Vec` grows by, in a crate that reads a
// `Vec`'s capacity: std asks room for its hint's lower bound, what's left
// of the inner iterator it's in, so its capacity is std's, not JS's own
// `flatMap`'s, which has no hint (#1528).

fn main() {
    let v = vec![1u32, 2, 3];

    // Stepped once, then collected.
    let mut fm = v.iter().flat_map(|&x| vec![x; 12]);
    fm.next();
    let rest: Vec<u32> = fm.collect();
    println!("{} {}", rest.len(), rest.capacity());

    // A flatten, stepped twice.
    let mut fl = v.iter().map(|&x| vec![x; 12]).flatten();
    fl.next();
    fl.next();
    let rest: Vec<u32> = fl.collect();
    println!("{} {}", rest.len(), rest.capacity());

    // Through a `skip(n)`, collected where it's made.
    let rest: Vec<u32> = v.iter().flat_map(|&x| vec![x; 12]).skip(3).collect();
    println!("{} {}", rest.len(), rest.capacity());

    // Collected at once.
    let all: Vec<u32> = v.iter().flat_map(|&x| vec![x; 5]).collect();
    println!("{} {}", all.len(), all.capacity());

    // An `extend` of a `Vec` that has room for some.
    let mut grown = Vec::with_capacity(3);
    grown.push(0u32);
    let mut it = v.iter().flat_map(|&x| vec![x; 7]);
    it.next();
    grown.extend(it);
    println!("{} {}", grown.len(), grown.capacity());

    // Of a range, a `$Steps` whose own `flatMap` knows no inner's hint.
    let mut ranged = (0u32..3).flat_map(|x| vec![x; 12]);
    ranged.next();
    let rest: Vec<u32> = ranged.collect();
    println!("{} {}", rest.len(), rest.capacity());

    // A `rev()` of one a `let` holds, stepped from its back.
    let mut back = (0u32..3).flat_map(|x| vec![x; 12]).rev();
    back.next();
    let rest: Vec<u32> = back.collect();
    println!("{} {}", rest.len(), rest.capacity());

    // A `chain`'s second part, whose hint the chain sums: one a `let`
    // holds, stepped first, and one made there.
    let mut stepped = (0u32..3).flat_map(|x| vec![x; 12]);
    stepped.next();
    let rest: Vec<u32> = [9u32].iter().copied().chain(stepped).collect();
    println!("{} {}", rest.len(), rest.capacity());
    let rest: Vec<u32> = [9u32].iter().copied().chain(v.iter().flat_map(|&x| vec![x; 12])).collect();
    println!("{} {}", rest.len(), rest.capacity());

    // A `zip`'s second part, whose hint the zip takes the fewer of.
    let mut paired = v.iter().flat_map(|&x| vec![x; 12]);
    paired.next();
    let rest: Vec<(u32, u32)> = (0u32..30).zip(paired).collect();
    println!("{} {}", rest.len(), rest.capacity());

    // Of inner iterators of different lengths.
    let words = ["a", "bcd", "efghij"];
    let mut chars = words.iter().flat_map(|w| w.chars().collect::<Vec<_>>());
    chars.next();
    chars.next();
    let rest: Vec<char> = chars.collect();
    let shown: String = rest.iter().collect();
    println!("{shown} {} {}", rest.len(), rest.capacity());
}
