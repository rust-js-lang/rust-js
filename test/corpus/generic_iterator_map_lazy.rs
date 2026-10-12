// A lazy iterator given generic code is lazy there: `map`'s closure runs
// as each item is asked for.
fn after_first<I: Iterator<Item = u32>>(mut it: I) {
    it.next();
    for x in it {
        println!("took {x}");
        if x >= 2 {
            break;
        }
    }
}

fn main() {
    after_first((1..5).map(|n| {
        println!("made {n}");
        n
    }));
}
