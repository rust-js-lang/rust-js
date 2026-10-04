//@ run-fail: called `Result::unwrap()` on an `Err` value: Missing { name: "ink", shelf: Some(2) }
// `unwrap()` of an `Err` shows the error by its own `Debug`, its type's name
// and all, as Rust does.

#[derive(Debug)]
enum StockError {
    Missing { name: String, shelf: Option<u32> },
}

fn find(name: &str) -> Result<u32, StockError> {
    match name {
        "pen" => Ok(3),
        _ => Err(StockError::Missing { name: name.to_string(), shelf: Some(2) }),
    }
}

fn main() {
    println!("{}", find("pen").unwrap());
    println!("{}", find("ink").unwrap());
}
