//! A library whose writers take no `Formatter` options (ADR 0137): each
//! writes what it shows with `write!`, padding nothing, showing nothing
//! pretty, so its manifest says its writers are plain.
use std::fmt;

pub struct Meters(pub f64);

impl fmt::Display for Meters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}m", self.0)
    }
}

pub enum Unit {
    Metric,
    Imperial,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unit::Metric => write!(f, "metric"),
            Unit::Imperial => write!(f, "imperial"),
        }
    }
}
