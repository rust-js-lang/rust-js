// `?` returns an `Err` or a `None` early and gives what's inside
// otherwise (ADR 0052), from MIR as from THIR: an early `return`, as a
// person writes it.
#[derive(Debug)]
struct Odd(u32);

fn half(n: u32) -> Result<u32, Odd> {
    if n % 2 == 0 { Ok(n / 2) } else { Err(Odd(n)) }
}

fn quarter(n: u32) -> Result<u32, Odd> {
    let h = half(n)?;
    let q = half(h)?;
    Ok(q)
}

fn first_doubled(v: &[u32]) -> Option<u32> {
    let x = v.first()?;
    Some(*x * 2)
}

fn main() {
    println!("{:?} {:?} {:?}", quarter(8), quarter(6), quarter(5));
    println!("{:?} {:?}", first_doubled(&[4, 5]), first_doubled(&[]));
}
