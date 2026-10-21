use dep::Ext;

pub fn main() {
    println!("{:?}", (1..4).skipped().collect::<Vec<_>>());
    println!("{:?}", (1..10).every(3).collect::<Vec<_>>());
    println!("{:?}", vec!["a", "b", "c"].into_iter().every(2).collect::<Vec<_>>());
}
