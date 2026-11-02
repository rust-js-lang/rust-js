// A `filter_map` held in a variable that a map stepped from its back steps
// (#218): made knowing its back where it's made, a map filtered,
// `$filterEnds($mapEnds(..))`, so the map, std's struct where it's held
// (ADR 0871), is given its `next_back` (ADR 0545, amended, #473).
fn main() {
    let v = vec![1, 2, 3, 4, 5, 6];
    let tripled = v.iter().filter_map(|x| if x % 2 == 0 { Some(x * 3) } else { None });
    let mut more = tripled.map(|x| x + 1);
    println!("{:?} {:?}", more.next_back(), more.next());
}
