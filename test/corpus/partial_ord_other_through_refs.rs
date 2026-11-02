//@ library-differs
// A type compared with another of its kind through references, `&At<Z>
// >= &At<Utc>` in code generic over `Z`, as chrono's `map_local` filters
// by `dt >= &DateTime::<Utc>::MIN_UTC`: std's `impl PartialOrd<&B> for
// &A` is its referents' order, the crate's `PartialOrd<At<Z2>> for
// At<Z>` (#467).

use std::cmp::Ordering;

pub trait Zone {}

#[derive(Debug)]
pub struct Utc;
impl Zone for Utc {}

#[derive(Debug)]
pub struct Fixed(i32);
impl Zone for Fixed {}

#[derive(Debug)]
pub struct At<Z: Zone> {
    t: i64,
    zone: Z,
}

impl<Z: Zone, Z2: Zone> PartialEq<At<Z2>> for At<Z> {
    fn eq(&self, other: &At<Z2>) -> bool {
        self.t == other.t
    }
}

impl<Z: Zone, Z2: Zone> PartialOrd<At<Z2>> for At<Z> {
    fn partial_cmp(&self, other: &At<Z2>) -> Option<Ordering> {
        self.t.partial_cmp(&other.t)
    }
}

const MIN: At<Utc> = At { t: 0, zone: Utc };
const MAX: At<Utc> = At { t: 100, zone: Utc };

pub fn in_range<Z: Zone>(at: Option<At<Z>>) -> Option<At<Z>> {
    at.filter(|at| at >= &MIN && at <= &MAX)
}

fn before<Z: Zone>(a: &At<Z>, b: &At<Utc>) -> (bool, bool) {
    (a < b, a > b)
}

fn main() {
    let kept = in_range(Some(At { t: 5, zone: Fixed(1) }));
    println!("{kept:?} {:?}", kept.as_ref().map(|at| at.zone.0));
    println!("{:?}", in_range(Some(At { t: -5, zone: Fixed(2) })));
    println!("{:?}", in_range(Some(At { t: 500, zone: Fixed(3) })));
    println!("{:?}", before(&At { t: 1, zone: Fixed(4) }, &MAX));
    println!("{:?}", before(&At { t: 101, zone: Fixed(5) }, &MAX));
    println!("{:?} {:?}", MIN.zone, MAX.zone);
}
