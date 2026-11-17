// A `str::split` or `rsplit` collected into a `Vec` straight from its
// making is never empty: text without its separator, `""` too, is given
// whole. So its `last()` and `first()` are `Some`, and `unwrap()` of them
// gives the item with no check (#1860). Where the `Split` was stepped
// before its `collect`, or the `Vec` emptied after, or the iterator is a
// `split_terminator`'s or a `splitn(0, ..)`'s, which may give none, the
// check stays, and `None` is what native Rust has.
fn last_key(path: &str) -> &str {
    let keys: Vec<&str> = path.split('.').collect();
    keys.last().unwrap()
}

fn first_word(text: &str) -> &str {
    let words: Vec<&str> = text.rsplit(' ').collect();
    words.first().unwrap()
}

fn stepped_first(path: &str) -> Option<String> {
    let mut parts = path.split('.');
    parts.next();
    let rest: Vec<&str> = parts.collect();
    rest.last().map(|s| s.to_string())
}

fn stepped_last(path: &str) -> String {
    let mut parts = path.split('.');
    parts.next();
    let rest: Vec<&str> = parts.collect();
    rest.last().unwrap().to_string()
}

fn terminated_last(text: &str) -> String {
    let pieces: Vec<&str> = text.split_terminator(',').collect();
    pieces.last().unwrap().to_string()
}

fn emptied(path: &str) -> String {
    let mut keys: Vec<&str> = path.split('.').collect();
    keys.clear();
    keys.last().copied().unwrap_or("none").to_string()
}

fn emptied_last(path: &str) -> String {
    let mut keys: Vec<&str> = path.split('.').collect();
    keys.clear();
    keys.last().unwrap().to_string()
}

fn terminated(text: &str) -> Option<String> {
    let pieces: Vec<&str> = text.split_terminator(',').collect();
    pieces.last().map(|s| s.to_string())
}

fn none_wanted(text: &str) -> Option<String> {
    let pieces: Vec<&str> = text.splitn(0, ',').collect();
    pieces.last().map(|s| s.to_string())
}

fn main() {
    println!("{} {} {}", last_key("a.b.c"), last_key("plain"), last_key(""));
    println!("{:?} {:?}", first_word("one two three"), first_word(""));
    println!("{:?} {:?}", stepped_first("a.b"), stepped_first("only"));
    println!("{} {}", stepped_last("a.b"), terminated_last("a,b,"));
    println!("{}", emptied("x.y"));
    println!("{}", std::panic::catch_unwind(|| emptied_last("x.y")).is_err());
    println!("{:?} {:?}", terminated("a,b,"), terminated(""));
    println!("{:?}", none_wanted("a,b"));
}
