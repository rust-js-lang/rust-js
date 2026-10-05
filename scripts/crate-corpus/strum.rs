use std::str::FromStr;

use strum::{AsRefStr, Display, EnumCount, EnumIter, EnumString, IntoEnumIterator, VariantNames};

#[derive(Debug, Clone, Copy, PartialEq, Display, EnumString, EnumIter, EnumCount, AsRefStr, VariantNames)]
#[strum(serialize_all = "snake_case")]
pub enum Color {
    Red,
    DarkGreen,
    #[strum(serialize = "blue", serialize = "b")]
    Blue,
}

#[derive(Debug, PartialEq, Display, EnumString)]
pub enum Shape {
    #[strum(to_string = "circle of {radius}")]
    Circle { radius: u32 },
    #[strum(ascii_case_insensitive)]
    Square,
}

pub fn report() -> String {
    let mut lines = Vec::new();
    let all: Vec<String> = Color::iter().map(|c| format!("{c}/{}/{c:?}", c.as_ref())).collect();
    lines.push(all.join(" "));
    lines.push(format!("{} {:?}", Color::COUNT, Color::VARIANTS));
    for text in ["red", "dark_green", "b", "blue", "Red", "green"] {
        match Color::from_str(text) {
            Ok(c) => lines.push(format!("{text} -> {c:?}")),
            Err(e) => lines.push(format!("{text} -> error {e}")),
        }
    }
    lines.push(format!("{} {}", Shape::Circle { radius: 3 }, Shape::Square));
    lines.push(format!(
        "{:?} {:?}",
        Shape::from_str("SQUARE"),
        Shape::from_str("Circle")
    ));
    lines.join("\n")
}
