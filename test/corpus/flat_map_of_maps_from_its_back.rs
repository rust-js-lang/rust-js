// A `flat_map()` whose closure gives a `map()` of each group, stepped from
// its back: the closure's `map()` is made in place, as what `flat_map`
// steps where it's given, and read by its form, `$mapEnds`, never taken
// for std's compiled `Map` by the call's span, whose back it doesn't have
// (#716).
fn main() {
    let groups = vec![vec![1, 2], vec![], vec![3, 4, 5]];
    let last = groups.iter().flat_map(|g| g.iter().map(|x| x * 10)).next_back();
    println!("{last:?}");
    let back: Vec<i32> = groups.iter().flat_map(|g| g.iter().map(|x| x + 1)).rev().collect();
    println!("{back:?}");
    let folded = groups
        .iter()
        .flat_map(|g| g.iter().map(|x| x * 2))
        .rfold(String::new(), |s, x| s + &x.to_string());
    println!("{folded}");
    let nth = groups.iter().flat_map(|g| g.iter().map(|x| x * 3)).nth_back(1);
    println!("{nth:?}");
    let found = groups.iter().flat_map(|g| g.iter().map(|x| x * 5)).rfind(|&x| x < 15);
    println!("{found:?}");
    let enumerated = groups
        .iter()
        .flat_map(|g| g.iter().enumerate().map(|(i, x)| i as i32 + x))
        .next_back();
    println!("{enumerated:?}");
    let reversed = groups.iter().flat_map(|g| g.iter().rev().map(|x| x * 7)).next_back();
    println!("{reversed:?}");
}
