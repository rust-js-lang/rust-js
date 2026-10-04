// A type's own `Hash`, as semver's and arrayvec's are: a map keys by value,
// as it does a derived one's, and never calls `hash`, so the impl, which
// Rust requires to agree with `==`, changes nothing a program sees.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(PartialEq, Eq, Debug, Clone)]
struct Version {
    major: u32,
    minor: u32,
    label: String,
}

// Hashes fewer fields than `==` compares: equal versions hash the same.
impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.major.hash(state);
        state.write_u32(self.minor);
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
enum Channel {
    Stable,
    Beta(u8),
}

impl Hash for Channel {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Channel::Stable => 0u8.hash(state),
            Channel::Beta(n) => n.hash(state),
        }
    }
}

// One impl for each of a generic type's arguments: no JS of either, so
// no names to collide.
#[derive(PartialEq, Eq, Debug)]
struct Tagged<T>(T);

impl Hash for Tagged<u8> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(self.0);
    }
}

impl Hash for Tagged<char> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

fn counts(items: &[Channel]) -> HashMap<Channel, usize> {
    let mut map = HashMap::new();
    for item in items {
        *map.entry(*item).or_insert(0) += 1;
    }
    map
}

fn main() {
    let v = |major, minor, label: &str| Version { major, minor, label: label.to_string() };
    let mut released = HashMap::new();
    released.insert(v(1, 0, "a"), "first");
    released.insert(v(1, 0, "b"), "same numbers, another label");
    released.insert(v(1, 0, "a"), "first, again");
    println!("{} {:?} {:?}", released.len(), released.get(&v(1, 0, "a")), released.get(&v(2, 0, "a")));

    let seen: HashSet<Channel> = [Channel::Stable, Channel::Beta(2), Channel::Beta(2), Channel::Beta(3)].into_iter().collect();
    println!("{} {} {}", seen.len(), seen.contains(&Channel::Beta(3)), seen.contains(&Channel::Beta(4)));

    let tally = counts(&[Channel::Beta(1), Channel::Stable, Channel::Beta(1)]);
    println!("{:?} {:?}", tally.get(&Channel::Beta(1)), tally.get(&Channel::Stable));
    let bytes: HashSet<Tagged<u8>> = [Tagged(1), Tagged(1), Tagged(2)].into_iter().collect();
    let letters: HashSet<Tagged<char>> = [Tagged('a')].into_iter().collect();
    println!("{} {}", bytes.len(), letters.contains(&Tagged('a')));
    let mut by_version: HashMap<Version, usize> = HashMap::new();
    for version in [v(0, 1, "x"), v(0, 1, "x"), v(0, 1, "y")] {
        *by_version.entry(version).or_insert(0) += 1;
    }
    println!("{} {:?}", by_version.len(), by_version.get(&v(0, 1, "x")));
}
