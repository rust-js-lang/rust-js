use dep::{Id, Version};

pub fn main() {
    let mut ids = vec![Id([2, 0]), Id([1, 9]), Id([1, 2])];
    ids.sort();
    println!("{ids:?} {:?}", Id([1, 0]).max(Id([0, 9])));
    let a = Version { major: 1, minor: 0.5 };
    let b = Version { major: 1, minor: f64::NAN };
    println!("{:?} {:?} {}", a.partial_cmp(&a.clone()), a.partial_cmp(&b), a < Version { major: 2, minor: 0.0 });
}
