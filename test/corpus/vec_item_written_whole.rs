// `v[i] = p` of a `Vec` of objects is the item's place written, `v[$at(v,
// i)] = p`, as a slice's is: not the old object made `p` in place, which
// another holder of it would see.

#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Named {
    name: String,
}

fn replaced(points: &mut Vec<Point>, i: usize) {
    points[i] = Point { x: 9.0, y: -1.0 };
    points.push(points[i]);
}

fn main() {
    let origin = Point { x: 0.0, y: 0.0 };
    let mut points = vec![origin, origin, origin];
    replaced(&mut points, 1);
    points[2] = Point { x: 3.0, y: 4.0 };
    let shown: Vec<String> = points.iter().map(|p| format!("({}, {})", p.x, p.y)).collect();
    println!("({}, {}) {shown:?}", origin.x, origin.y);

    let mut names = vec![Named { name: "a".to_string() }, Named { name: "b".to_string() }];
    let first = Named { name: "c".to_string() };
    names[0] = first;
    names.push(Named { name: "d".to_string() });
    let shown: Vec<&str> = names.iter().map(|n| n.name.as_str()).collect();
    println!("{shown:?}");

    let mut louds = vec![Loud(1), Loud(2)];
    louds[0] = Loud(3);
    println!("set");
    louds.push(Loud(4));
    println!(
        "{}",
        louds.iter().map(|l| l.0.to_string()).collect::<Vec<_>>().join(" ")
    );
}
