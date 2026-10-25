// A `chain` collected into what nothing reads, `[...a, ...b]`, is still
// made, each side stepped as Rust steps it: the array whole, not its
// spreads taken apart into statements (#256).
fn probe(n: u32) -> u32 {
    println!("probe {n}");
    n
}

fn main() {
    let _seen: Vec<u32> = (0..3).map(probe).chain(vec![9]).collect();
    let _plain: Vec<i32> = vec![1, 2].into_iter().chain(vec![3]).collect();
    println!("end");
}
