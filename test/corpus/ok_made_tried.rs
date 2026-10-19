// `?` of an `Ok` made where it's tried: no `Err` to give back, so no test,
// and what it holds read as it is, as a person writes it.

fn made() -> Result<u32, String> {
    let v = Ok::<u32, String>(3)?;
    Ok(v + 1)
}

fn checked(n: u32) -> Result<u32, String> {
    let half = if n % 2 == 0 {
        Ok(n / 2)
    } else {
        Err(format!("{n} is odd"))
    }?;
    let unit: Result<(), String> = Ok(());
    unit?;
    Ok(half)
}

fn main() {
    println!("{:?} {:?} {:?}", made(), checked(8), checked(3));
}
