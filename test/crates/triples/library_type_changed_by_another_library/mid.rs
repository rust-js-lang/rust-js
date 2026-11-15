//! Changes `dep`'s `Spot` in place, which `dep` itself never does: what
//! it's given by value, and through a `&mut`, and a `Vec` of its own.
use dep::Spot;

pub fn nudged(mut spot: Spot) -> Spot {
    spot.x += 1;
    spot
}

pub fn shift(spot: &mut Spot) {
    spot.y += 10;
}

pub fn pushed(mut names: Vec<String>) -> Vec<String> {
    names.push("pushed".to_string());
    names
}
