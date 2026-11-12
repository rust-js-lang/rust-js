// `split_once` and `rsplit_once` by a closure ask it of each `char` as std's
// `find` and `rfind` do: from the front, or from the back, until it holds,
// and of none past it. `split_once` asked it of every `char`, and
// `rsplit_once` searched from the front (#1501).
fn main() {
    println!(
        "{:?}",
        "x=y=z".split_once(|c: char| {
            println!("so {c}");
            c == '='
        })
    );
    println!(
        "{:?}",
        "x=y=z".rsplit_once(|c: char| {
            println!("rso {c}");
            c == '='
        })
    );
    println!("{:?}", "née=ü".split_once(|c: char| c == '='));
    println!("{:?}", "née=ü".rsplit_once(|c: char| c == 'é'));
    println!(
        "{:?}",
        "abc".split_once(|c: char| {
            println!("none {c}");
            c == '='
        })
    );
    println!("{:?}", "abc".rsplit_once(char::is_numeric));
}
