// `strip_circumfix(prefix, suffix)`: `strip_prefix`'s, then `strip_suffix`'s
// of what's left, each once, of a `&str` or a `char` pattern.
fn main() {
    println!("{:?}", "bar:hello:foo".strip_circumfix("bar:", ":foo"));
    println!("{:?}", "bar:foo".strip_circumfix("foo", "foo"));
    println!("{:?}", "foo:bar;".strip_circumfix("foo:", ';'));
    println!("{:?}", "foo:bar:baz".strip_circumfix("foo:bar:", ":bar:baz"));
    println!("{:?}", "[]".strip_circumfix('[', ']'));
    println!("{:?}", "[".strip_circumfix('[', '['));
    let quoted = String::from("«héllo»");
    println!("{:?}", quoted.strip_circumfix('«', '»').map(str::len));
}
