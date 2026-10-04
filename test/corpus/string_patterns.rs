// Splitting and searching by a pattern, as Rust does: `rsplit` searches from
// the end, so overlapping matches split where Rust's do; offsets are UTF-8
// bytes; an empty pattern matches between each `char`.

fn main() {
    let line = "a=b=c";
    println!("{:?} {:?}", line.splitn(2, '=').collect::<Vec<_>>(), line.splitn(1, '=').collect::<Vec<_>>());
    println!("{:?} {:?}", line.rsplitn(2, '=').collect::<Vec<_>>(), line.rsplit('=').collect::<Vec<_>>());
    println!("{:?} {:?}", "aaa".rsplit("aa").collect::<Vec<_>>(), "aaaa".rsplitn(2, "aa").collect::<Vec<_>>());
    println!("{:?} {:?}", "ab".rsplit("").collect::<Vec<_>>(), "ab".splitn(2, "").collect::<Vec<_>>());
    println!("{:?} {:?}", "a,b,".split_terminator(',').collect::<Vec<_>>(), "".split_terminator(",").collect::<Vec<_>>());
    println!("{:?}", "a,,b,,".split_terminator(",,").collect::<Vec<_>>());
    let (left, right) = "héllo wörld".split_at(6);
    println!("{left:?} {right:?}");
    println!("{:?}", "héabcéabc".match_indices("bc").collect::<Vec<_>>());
    println!("{:?}", "ab".match_indices("").collect::<Vec<_>>());
    println!("{} {:?}", "banana".matches("an").count(), "a-b-c".matches('-').collect::<Vec<_>>());
    println!("{:?} {:?}", "xxhixx".trim_matches('x'), "--a--".trim_start_matches('-'));
    println!("{:?} {:?}", "xxhixx".trim_start_matches("xx"), "a1b22".trim_end_matches(char::is_numeric));
    println!("{:?}", " a1 ".trim_matches(|c: char| c == ' ' || c.is_ascii_digit()));
    let mut total = 0;
    for part in "k=v;x=y".split_terminator(';') {
        let mut kv = part.splitn(2, '=');
        total += kv.next().unwrap().len() + kv.next().unwrap_or("").len();
    }
    println!("{total}");
}
