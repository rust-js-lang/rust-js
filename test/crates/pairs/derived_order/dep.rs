#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(pub [u8; 2]);

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Version {
    pub major: u32,
    pub minor: f64,
}
