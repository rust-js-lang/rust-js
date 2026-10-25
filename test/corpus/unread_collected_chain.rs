// A `chain` collected into what nothing reads, `[...a, ...b]`, is still
// made, each side stepped as Rust steps it: the array whole, not its
// spreads taken apart into statements (#256), nor the array taken away as
// if a spread read only variables, in a loop too, where its `inspect` runs
// (#273).
fn probe(n: u32) -> u32 {
    println!("probe {n}");
    n
}

fn main() {
    let _seen: Vec<u32> = (0..3).map(probe).chain(vec![9]).collect();
    let _plain: Vec<i32> = vec![1, 2].into_iter().chain(vec![3]).collect();
    for round in 0..probe(1) {
        let _again: Vec<u32> = vec![probe(round), probe(round + 1)]
            .into_iter()
            .inspect(|seen| println!("see {seen}"))
            .chain(vec![probe(9)])
            .collect();
    }
    println!("end");
}
