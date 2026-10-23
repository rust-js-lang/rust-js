// A slice's binary_search of items `<` doesn't order as Ord does, a
// derived enum's, a struct's or text, is by their Ord, as sort() is; and
// dedup_by_key of keys `===` doesn't compare, by their `==`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Low,
    Mid(u8),
    High { name: String },
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version(u32, u32, &'static str);

fn main() {
    let mut levels = vec![
        Level::High { name: "b".to_string() },
        Level::Mid(3),
        Level::Low,
        Level::Mid(1),
        Level::High { name: "a".to_string() },
    ];
    levels.sort();
    for probe in [Level::Low, Level::Mid(2), Level::Mid(3), Level::High { name: "ab".to_string() }] {
        println!("{probe:?} {:?}", levels.binary_search(&probe));
    }
    let versions = [Version(1, 0, "a"), Version(1, 2, "b"), Version(2, 0, "")];
    println!("{:?} {:?}", versions.binary_search(&Version(1, 2, "b")), versions.binary_search(&Version(1, 3, "")));
    let words = ["apple", "banana", "cherry", "éclair"];
    println!("{:?} {:?} {:?}", words.binary_search(&"cherry"), words.binary_search(&"date"), words.binary_search(&"zebra"));
    let owned: Vec<String> = words.iter().map(|w| w.to_string()).collect();
    println!("{:?}", owned.binary_search(&"banana".to_string()));

    let mut names = vec!["apple", "avocado", "banana", "blueberry", "cherry", "", ""];
    names.dedup_by_key(|s| s.chars().next());
    println!("{names:?}");
    let mut levels = vec![Level::Mid(1), Level::Mid(2), Level::Low, Level::High { name: "x".into() }, Level::High { name: "y".into() }];
    levels.dedup_by_key(|l| match l {
        Level::High { .. } => Some("high".to_string()),
        Level::Mid(_) => Some("mid".to_string()),
        Level::Low => None,
    });
    println!("{levels:?}");
    let mut versions = vec![Version(1, 0, "a"), Version(1, 0, "b"), Version(1, 2, "c"), Version(1, 0, "d")];
    versions.dedup_by_key(|v| (v.0, v.1));
    println!("{versions:?}");
}
