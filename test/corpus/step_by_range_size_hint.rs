// A range's `step_by` given to generic code is told its size, std's
// `StepBy`'s of its range's exact length: before its first item is taken,
// that one and each `step`th after; then each `step`th.
fn hint<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn start(n: u32) -> u32 {
    n
}

fn main() {
    let r = start(1)..start(11);
    println!("{:?}", hint(r.clone().step_by(3)));
    println!("{:?}", hint(r.clone().step_by(20)));
    let mut stepped = r.step_by(4);
    stepped.next();
    println!("{:?}", hint(stepped));
    println!("{:?}", hint((start(5)..start(5)).step_by(2)));
}
