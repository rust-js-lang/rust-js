// A reversed chain stepped from its back, `rev().rfold(..)`, is its
// source stepped from its front: the `rev()` given to the back stepper is
// what it steps, never turned into a chain over reversed items, so a
// `Vec<String>`'s `into_iter().map(f).rev().rfold(..)` folds as std's
// does, where it threw "it[by] is not a function" (#1855).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn words() -> Vec<String> {
    vec![String::from("1"), String::from("2"), String::from("3")]
}

fn made(s: String) -> D {
    D(s.parse::<u32>().unwrap() + 100)
}

fn main() {
    let n = words().into_iter().map(made).rev().rfold(0, |a, d| a + d.0);
    println!("rfold {n}");
    let x = words().into_iter().map(made).rev().nth_back(1).map(|d| d.0);
    println!("nth_back {x:?}");
    let x = words().into_iter().map(made).rev().rfind(|d| d.0 > 101).map(|d| d.0);
    println!("rfind {x:?}");
    let n: Option<u32> = words().into_iter().map(made).rev().try_rfold(0u32, |a, d| a.checked_add(d.0));
    println!("try_rfold {n:?}");
    let n = words().iter().map(|s| D(s.len() as u32 + 100)).rev().rfold(0, |a, d| a + d.0);
    println!("of references {n}");
    let n = vec![D(1), D(2)].into_iter().map(|d| D(d.0 + 100)).rev().rfold(0, |a, d| a + d.0);
    println!("of ds {n}");
}
