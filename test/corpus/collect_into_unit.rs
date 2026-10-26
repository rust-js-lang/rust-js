// `collect()` into `()`: each item stepped, as std's body,
// `iter.for_each(|()| {})`, steps them; of a `map`, its function run for
// each, `forEach`, as the autocomplete trie gathers its words.
fn words(prefix: &str, rest: &[&str]) -> Vec<String> {
    let mut out = vec![];
    if rest.is_empty() {
        out.push(prefix.to_string())
    } else {
        rest.iter().map(|s| out.push(prefix.to_string() + s)).collect()
    }
    out
}

fn main() {
    println!("{:?}", words("re", &["ad", "do", ""]));
    let mut total = 0;
    let () = (1..=4).map(|i| total += i).collect();
    let mut seen = Vec::new();
    let lazy = [3, 1, 2].iter().map(|n| seen.push(n * 10));
    let () = lazy.collect();
    println!("{total} {seen:?}");
}
