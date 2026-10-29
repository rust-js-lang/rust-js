// A zip's `size_hint()` is std's `Zip`'s: the fewer of its sides' lower
// bounds, and of their upper, the one either has, or none (#400). Made in
// place, its sides' each asked as it's made, a JS iterator's too, a
// `skip()`'s, a `map()`'s or an owned `Vec`'s; of a range, an endless
// one, and reversed; given to generic code, of what its sides' JS tells;
// and std's struct, held and stepped.

fn hint<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn show((low, high): (usize, Option<usize>)) {
    println!("{low} {high:?}");
}

fn main() {
    let v = vec![1, 2, 3, 4, 5];
    let w = vec![7, 8];
    show(v.iter().zip(v.iter()).size_hint());
    show(v.iter().zip(v.iter().skip(2)).size_hint());
    show(v.iter().zip(v[2..].iter()).size_hint());
    show(v.clone().into_iter().zip(w.clone()).size_hint());
    show((0..4).zip(0..9).size_hint());
    show(v.iter().zip(v.iter().map(|x| x * 2)).size_hint());
    show(v.iter().zip(w.iter()).rev().size_hint());
    show((0..).zip(w.iter()).size_hint());

    show(
        v.iter()
            .map(|x| {
                println!("unrun {x}");
                x * 2
            })
            .zip(w.iter())
            .size_hint(),
    );
    let items = v.iter();
    show(items.zip(w.iter().chain(w.iter())).size_hint());
    show(hint(v.iter().zip(w.iter())));
    show(hint((0..).zip(w.iter())));

    let mut held = v.iter().zip(w.iter().map(|x| x + 1));
    show(held.size_hint());
    held.next();
    show(held.size_hint());
    held.next();
    held.next();
    show(held.size_hint());

    let mut pairs = v
        .iter()
        .map(|x| {
            println!("step {x}");
            x * 10
        })
        .zip(w.iter());
    println!("{:?}", pairs.next());
    show(pairs.size_hint());
}
