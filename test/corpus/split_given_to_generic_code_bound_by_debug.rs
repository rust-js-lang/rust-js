// A `str`'s `split` given to generic code bound by `Debug` that never shows
// it: compiled, as a bound nothing reads is given no dictionary (ADR 1165,
// #1687). Its `{:?}` itself, std's searcher's state, is refused (#2004).
fn passed<I: Iterator + std::fmt::Debug>(it: I) -> I {
    it
}

fn counted<I: Iterator + std::fmt::Debug + Clone>(it: I) -> usize {
    it.clone().count() + it.count()
}

fn main() {
    let words: Vec<&str> = passed("a b c".split(' ')).collect();
    println!("{words:?}");
    println!("{}", counted("x,y".split(',')));
    let found: Vec<(usize, &str)> = passed("abab".match_indices("ab")).collect();
    println!("{found:?}");
}
