use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use std::str::FromStr;

pub fn report() -> String {
    let mut lines = Vec::new();
    let values: Vec<Decimal> = [
        "1.10",
        "2.205",
        "-0.5",
        "100",
        "0.000001",
        "79228162514264337593543950335",
    ]
    .iter()
    .map(|s| Decimal::from_str(s).unwrap())
    .collect();
    for d in &values {
        lines.push(format!(
            "{d} scale {} neg {} round2 {} floor {} ceil {} trunc {} f64 {:?}",
            d.scale(),
            d.is_sign_negative(),
            d.round_dp(2),
            d.floor(),
            d.ceil(),
            d.trunc(),
            d.to_f64(),
        ));
    }
    let a = Decimal::from_str("1.1").unwrap();
    let b = Decimal::from_str("2.2").unwrap();
    lines.push(format!("{} {} {} {}", a + b, b - a, a * b, b / a));
    lines.push(format!(
        "{:?} {:?}",
        a.checked_div(Decimal::ZERO),
        values[5].checked_add(Decimal::ONE)
    ));
    lines.push(format!("{} {}", Decimal::new(12345, 3), Decimal::from_i64(-7).unwrap()));
    let mut sorted = values.clone();
    sorted.sort();
    lines.push(format!("{sorted:?}"));
    lines.push(format!("{:?}", Decimal::from_str("abc")));
    lines.join("\n")
}
