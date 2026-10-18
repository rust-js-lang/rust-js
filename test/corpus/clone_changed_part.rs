// A struct whose part is changed through a `&mut` of it, `copy[0].0.push(..)`,
// is copied where it's cloned: the part's text is written into it,
// `copy[0][0] += "?"`, so the clone mustn't share it with the original
// (ADR 0052).
#[derive(Clone, Debug)]
struct Token(String);

#[derive(Clone, Debug)]
struct Counter {
    count: u32,
}

fn bump(n: &mut u32) {
    *n += 1;
}

fn main() {
    let tokens = vec![Token("a".into()), Token("b".into())];
    let mut copy = tokens.clone();
    copy[0].0.push('?');
    println!("{tokens:?} {copy:?}");
    let counters = [Counter { count: 1 }];
    let mut changed = counters.clone();
    bump(&mut changed[0].count);
    println!("{counters:?} {changed:?}");
}
