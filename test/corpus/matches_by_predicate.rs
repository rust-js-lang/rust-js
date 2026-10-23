// matches() and rmatches() of a predicate of a char, a closure, a function
// or a set of chars: each char it holds of, rmatches asking from the end.
fn main() {
    println!("{:?}", "AbC".matches(char::is_uppercase).collect::<Vec<_>>());
    println!("{:?}", "a1b22c".rmatches(|c: char| c.is_ascii_digit()).collect::<Vec<_>>());
    println!("{:?}", "x-y_z".matches(['-', '_']).count());
    println!("{:?}", "héllo".matches(&['é', 'o'][..]).collect::<String>());
    let mut seen = vec![];
    let n = "abc".rmatches(|c: char| {
        seen.push(c);
        c != 'b'
    }).count();
    println!("{n} {seen:?}");
}
