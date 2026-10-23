// A closure, a function or a set of `char`s as the pattern of each `str`
// method that takes one, of text whose `char`s are more than one UTF-8
// byte and more than one UTF-16 unit (#118).
fn main() {
    let s = "a1é2,b3;🦀4";
    let digit = |c: char| c.is_ascii_digit();
    println!("{:?}", s.rsplit(digit).collect::<Vec<_>>());
    println!("{:?}", s.rsplitn(2, char::is_numeric).collect::<Vec<_>>());
    println!("{:?}", s.rsplitn(0, digit).collect::<Vec<_>>());
    println!("{:?}", s.split_terminator([',', ';']).collect::<Vec<_>>());
    println!("{:?}", "x,y,".split_terminator(|c| c == ',').collect::<Vec<_>>());
    println!("{:?}", "x,y,".rsplit_terminator(&[','][..]).collect::<Vec<_>>());
    println!("{:?}", s.split_inclusive(digit).collect::<Vec<_>>());
    println!("{:?}", s.match_indices(char::is_numeric).collect::<Vec<_>>());
    println!("{:?}", s.rmatch_indices(|c: char| !c.is_ascii()).collect::<Vec<_>>());
    println!("{:?} {:?}", s.split_once(digit), s.rsplit_once([',', ';']));
    println!("{:?} {:?}", "ab".split_once(digit), s.strip_prefix(char::is_alphabetic));
    println!("{:?} {:?}", s.strip_suffix(digit), s.strip_suffix('x'));
    println!(
        "{} {}",
        s.replace(digit, "#"),
        s.replacen(|c: char| c.is_ascii_digit(), "", 2)
    );
    println!("{}", "hello world".replace(char::is_whitespace, "_"));
    println!("{:?}", "".split_terminator(digit).count());
    println!("{:?}", "".split_inclusive(digit).count());
    println!("{:?} {:?}", "1".strip_prefix(digit), "ab".strip_prefix(digit));
}
