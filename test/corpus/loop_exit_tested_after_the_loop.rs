// A `for` loop whose way out, once its items run out, tests a value of
// generic code before returning, as rand 0.7's `WeightedIndex::
// update_weights` does: `if total_weight == zero { return Err(..); }`
// after the loop, its other way on past it. Its `return` inside the loop
// and the test after it, each where Rust has it (#1422).
#[derive(Debug)]
enum WeightError {
    TooMany,
    AllZero,
}

fn update<X: PartialEq + Clone + Default>(total: &X, new_weights: &[(usize, &X)], len: usize) -> Result<(), WeightError> {
    let zero = X::default();
    let total_weight = total.clone();
    let mut prev_i = None;
    for &(i, _) in new_weights {
        if i >= len {
            return Err(WeightError::TooMany);
        }
        prev_i = Some(i);
    }
    println!("last {prev_i:?}");
    if total_weight == zero {
        return Err(WeightError::AllZero);
    }
    Ok(())
}

// A test of two, each a branch of its own whose `break` is inside the
// other: its jump stays in the loop.
fn first_missing<X: PartialEq + Default>(items: &[X], skip: &X) -> Option<usize> {
    let zero = X::default();
    for (i, item) in items.iter().enumerate() {
        if item != skip && *item == zero {
            return Some(i);
        }
    }
    if *skip == zero || items.is_empty() {
        return None;
    }
    Some(items.len())
}

fn main() {
    println!("{:?}", update(&1u32, &[(1, &2)], 3));
    println!("{:?}", update(&1u32, &[(1, &2), (5, &2)], 3));
    println!("{:?}", update(&0u32, &[], 3));
    println!("{:?}", update(&String::from("w"), &[(0, &String::new())], 1));
    println!("{:?}", first_missing(&[1, 0, 2], &5));
    println!("{:?}", first_missing(&[1, 2], &5));
    println!("{:?}", first_missing(&[1, 2], &0));
    println!("{:?}", first_missing::<i32>(&[], &5));
}
