//@ library-differs
// A program's errors: an enum of them with `Display`, `Error` and a `From`
// that `?` converts by, boxed as a `dyn Error` where they meet, and a
// `Result` collected (ADRs 0141, 0187).
use std::collections::HashMap;
use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
enum AppError {
    Parse(ParseIntError),
    Missing(String),
    Invalid { field: &'static str, value: i64 },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Parse(e) => write!(f, "parse: {e}"),
            AppError::Missing(k) => write!(f, "missing {k}"),
            AppError::Invalid { field, value } => write!(f, "{field} = {value} is invalid"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<ParseIntError> for AppError {
    fn from(e: ParseIntError) -> Self {
        AppError::Parse(e)
    }
}

fn field(config: &HashMap<&str, &str>, key: &str) -> Result<i64, AppError> {
    let raw = config.get(key).ok_or_else(|| AppError::Missing(key.to_string()))?;
    let n: i64 = raw.trim().parse()?;
    if n < 0 {
        return Err(AppError::Invalid {
            field: "count",
            value: n,
        });
    }
    Ok(n)
}

fn total(config: &HashMap<&str, &str>, keys: &[&str]) -> Result<i64, Box<dyn std::error::Error>> {
    let mut sum = 0;
    for k in keys {
        sum += field(config, k)?;
    }
    Ok(sum)
}

fn main() {
    let config: HashMap<&str, &str> = [("a", " 4"), ("b", "x"), ("c", "-2"), ("d", "10")]
        .into_iter()
        .collect();
    for keys in [&["a", "d"][..], &["a", "b"], &["c"], &["z"]] {
        match total(&config, keys) {
            Ok(n) => println!("ok {n}"),
            Err(e) => println!("error: {e}"),
        }
    }
    let results: Result<Vec<i64>, AppError> = ["a", "d"].iter().map(|k| field(&config, k)).collect();
    println!("{results:?}");
    let e = field(&config, "b").unwrap_err();
    println!("{e:?}");
}
