struct Point {
    x: f64,
    y: f64,
}

struct Bounds {
    width: f64,
    top: f64,
    left: f64,
}

fn read(name: &str, value: f64) -> f64 {
    print!("{name} ");
    value
}

fn total(point: &Point, bounds: &Bounds) -> f64 {
    point.x + point.y + bounds.width + bounds.top + bounds.left
}

fn total_array(point: &[f64; 2], bounds: &Bounds) -> f64 {
    point[0] + point[1] + bounds.width + bounds.top + bounds.left
}

fn main() {
    let value = total(
        &Point {
            x: read("x", 1.0),
            y: read("y", 2.0),
        },
        &Bounds {
            width: read("width", 3.0),
            top: read("top", 4.0),
            left: read("left", 5.0),
        },
    );
    println!("= {value}");
    let value = total_array(
        &[read("x", 1.0), read("y", 2.0)],
        &Bounds {
            width: read("width", 3.0),
            top: read("top", 4.0),
            left: read("left", 5.0),
        },
    );
    println!("= {value}");
}
