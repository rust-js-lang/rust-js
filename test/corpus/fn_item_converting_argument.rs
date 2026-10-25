// A function given as a value whose call gives an argument other than as
// it is, an `Option` an `IntoIterator` bound takes, is the arrow of its
// call, which does so too; one whose call gives each as it is stays its
// name (#281).

fn total<T: IntoIterator<Item = i32>>(items: T) -> i32 {
    let mut sum = 0;
    for x in items {
        sum += x;
    }
    sum
}

fn main() {
    let s: i32 = vec![Some(1), None, Some(3)].into_iter().map(total).sum();
    println!("{s}");
    let t: Vec<i32> = vec![vec![1, 2], vec![3]].into_iter().map(total).collect();
    println!("{t:?}");
    let f = total::<Option<i32>>;
    println!("{} {}", f(Some(4)), f(None));
}
