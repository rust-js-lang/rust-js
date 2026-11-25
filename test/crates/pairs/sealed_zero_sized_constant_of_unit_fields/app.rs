pub fn main() {
    let p = dep::pair(vec![1, 2, 3].into_iter());
    println!("{}", p.shown());
    let items: Vec<u32> = p.map(|n| n * 2).collect();
    println!("{items:?}");
}
