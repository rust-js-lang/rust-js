use dep::{counter, maybe_counter, said, ticker};

pub fn main() {
    let held = Some(counter());
    let copied = held;
    let cloned = held.clone();
    println!(
        "{} {} {}",
        held.map(|mut f| f()).unwrap(),
        copied.map(|mut f| f()).unwrap(),
        cloned.map(|mut f| f()).unwrap()
    );

    let mut m = maybe_counter();
    let m2 = m.clone();
    println!("{} {} {}", m.as_mut().unwrap()(), m.as_mut().unwrap()(), m2.map(|mut f| f()).unwrap());

    let t = ticker();
    let mut t2 = t;
    let mut t3 = t.clone();
    println!("{} {} {}", (t2.tick)(), (t2.tick)(), (t3.tick)());

    let mut s = said();
    let mut s2 = s.clone();
    let mut all = vec![said().0];
    let mut all2 = all.clone();
    println!("{} {} {} {} {}", (s.0)(), (s.0)(), (s2.0)(), all[0](), all2[0]());
}
