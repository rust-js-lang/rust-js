// `ptr::eq` of references read out of a `Vec` or a struct compares where
// they point, not the slots they were read from: the same item of another
// array is equal, another item isn't.
#[derive(Debug)]
struct Point {
    x: i32,
}

struct Pick<'a> {
    chosen: &'a Point,
}

fn main() {
    let points = vec![Point { x: 0 }, Point { x: 5 }];
    let mut refs: Vec<&Point> = Vec::new();
    refs.extend(&points);
    println!("{} {}", std::ptr::eq(refs[0], &points[0]), std::ptr::eq(refs[1], &points[0]));
    let pick = Pick { chosen: &points[1] };
    println!("{} {}", std::ptr::eq(pick.chosen, &points[1]), std::ptr::eq(pick.chosen, refs[0]));
    println!("{}", refs.iter().filter(|r| std::ptr::eq(**r, &points[1])).count());
    println!("{:?}", refs[0].x + pick.chosen.x);
}
