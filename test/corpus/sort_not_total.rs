//@ run-fail: user-provided comparison function does not correctly implement a total order
// A comparator that isn't a total order, `partial_cmp(..).unwrap_or(Equal)`
// of NaNs, is std's sort's too, not JS's (#936): any sort of a total order
// leaves one order, but of this one each sort leaves its own, and std's
// small sort's merge panics where it finds the halves inconsistent. Sorts
// of total orders, a key's, `total_cmp`'s, `then_with`'s, stay JS's.
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    major: u32,
    minor: u32,
}

/// Its own order, rock, paper and scissors: each beats the next, so it
/// isn't a total order, and each sort leaves its own.
#[derive(Debug, PartialEq, Eq)]
struct Hand(u32, u32);

impl PartialOrd for Hand {
    fn partial_cmp(&self, other: &Hand) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Hand {
    fn cmp(&self, other: &Hand) -> Ordering {
        match (self.0, other.0) {
            (0, 1) | (1, 2) | (2, 0) => Ordering::Less,
            (a, b) if a == b => Ordering::Equal,
            _ => Ordering::Greater,
        }
    }
}

fn main() {
    let mut hands: Vec<Hand> = (0..20).map(|i| Hand((i * i * 37 + 5) % 11 % 3, i)).collect();
    hands.sort();
    println!("{:?}", hands.iter().map(|h| h.1).collect::<Vec<_>>());
    let mut versions: Vec<Version> = (0..30)
        .map(|i| Version {
            major: i % 4,
            minor: i % 7,
        })
        .collect();
    versions.sort();
    versions.sort_by(|a, b| b.minor.cmp(&a.minor).then_with(|| a.major.cmp(&b.major)));
    println!("{:?}", versions.iter().map(|v| (v.major, v.minor)).collect::<Vec<_>>());

    let mut floats: Vec<f64> = (0..40).map(|i| ((i * 37) % 23) as f64 / 4.0).collect();
    floats.sort_by(f64::total_cmp);
    println!("{floats:?}");

    // Of these, std's order and JS's differ; then std's merge panics.
    let mut few: Vec<f64> = vec![3.0, f64::NAN, 1.0, 2.0, f64::NAN, 0.5];
    few.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    println!("{few:?}");
    let mut many: Vec<f64> = (0..40)
        .map(|i| if i % 7 == 3 { f64::NAN } else { ((i * 37) % 23) as f64 })
        .collect();
    many.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    println!("{many:?}");
}
