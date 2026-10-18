// A lock's `Err` is a `PoisonError` of its guard, which a lock never
// poisoned doesn't give: `unwrap_or_else(|e| e.into_inner())` of its `Ok`,
// just made, is the guard, as `unwrap()`'s is, uuid's clock sequence's
// (ADRs 0144, 0449).
use std::sync::{Mutex, PoisonError};

fn bump(m: &Mutex<u32>) -> u32 {
    let mut n = m.lock().unwrap_or_else(|e| e.into_inner());
    *n += 1;
    *n
}

fn name_len(m: &Mutex<String>) -> usize {
    m.lock().unwrap_or_else(PoisonError::into_inner).len()
}

fn held(m: &Mutex<Vec<u32>>) -> usize {
    let locked = m.lock();
    let peeked = match &locked {
        Ok(items) => items.len(),
        Err(e) => e.get_ref().len(),
    };
    peeked + locked.unwrap_or_else(|e| e.into_inner()).len()
}

fn loud() -> u32 {
    println!("made");
    0
}

fn main() {
    let count = Mutex::new(0);
    bump(&count);
    println!("{}", bump(&count));
    println!("{}", name_len(&Mutex::new(String::from("ada"))));
    println!("{}", held(&Mutex::new(vec![1, 2])));
    let five = u32::try_from(5u64).unwrap_or(0);
    let six = Ok::<u32, String>(6).unwrap_or_else(|e| e.len() as u32);
    let seven = Ok::<u32, String>(7).unwrap_or_default();
    let eight = Ok::<u32, String>(8).unwrap_or(0);
    let nine = Ok::<u32, String>(9).unwrap_or(loud());
    println!("{five} {six} {seven} {eight} {nine}");
}
