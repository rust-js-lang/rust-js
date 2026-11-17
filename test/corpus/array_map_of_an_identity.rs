// An array's `map` by a function that's the identity in JS, `String::from`
// of a `&str`, `|x| x` and `Into::into` between types of one
// representation: the array, with no `.map` (#1882). Where the array
// mapped is `Copy` and stays usable, and either array is written in place
// after, a copy of it, so writing the result leaves it as it was; where
// it's moved, or nothing writes either, the array itself.
fn labels() -> Vec<String> {
    ["Share", "Copy"].map(String::from).to_vec()
}

fn main() {
    println!("{:?}", labels());

    let a = ["x", "y"];
    let mut b = a.map(String::from);
    b[0].push_str("!");
    println!("{a:?} {b:?}");

    let c = ["p", "q"];
    let mut d = c.map(String::from);
    d[0] = String::from("z");
    println!("{c:?} {d:?}");

    let n = [1, 2, 3];
    let mut m = n.map(|x| x);
    m[0] = 5;
    println!("{n:?} {m:?}");

    let words = ["i", "j"];
    let mut into: [String; 2] = words.map(Into::into);
    into[1] = String::from("k");
    println!("{words:?} {into:?}");

    let owned = [String::from("o"), String::from("w")];
    let kept = owned.map(|s| s);
    println!("{kept:?}");

    let read = ["r", "s"];
    let shown = read.map(String::from);
    println!("{read:?} {shown:?}");
}
