// `*r = v` through a `&mut` to an untagged enum, whose value is its payload
// (ADR 0214): of one that may be a number, a place, written where it is, an
// array's item or a variable; of one whose payloads are each an object, the
// object, changed in place (ADR 0147). A matrix's cell, as
// trekhleb/javascript-algorithms' Matrix writes it.

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Debug)]
pub enum Matrix {
    Cell(f64),
    Rows(Vec<Matrix>),
}

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Debug)]
pub enum Shape {
    Points(Vec<f64>),
    Named(Named),
}

#[derive(Debug)]
pub struct Named {
    pub name: String,
}

fn update(m: &mut Matrix, indices: &[usize], value: f64) {
    let mut cell = m;
    for &index in indices {
        let Matrix::Rows(rows) = cell else {
            return;
        };
        cell = &mut rows[index];
    }
    *cell = Matrix::Cell(value);
}

fn grow(m: &mut Matrix) {
    if let Matrix::Cell(value) = *m {
        *m = Matrix::Rows(vec![Matrix::Cell(value), Matrix::Cell(value * 2.0)]);
    }
}

fn rename(shape: &mut Shape) {
    *shape = Shape::Named(Named {
        name: "renamed".to_string(),
    });
}

fn main() {
    let mut m = Matrix::Rows(vec![
        Matrix::Rows(vec![Matrix::Cell(0.0), Matrix::Cell(0.0)]),
        Matrix::Rows(vec![Matrix::Cell(1.0), Matrix::Cell(1.0)]),
    ]);
    update(&mut m, &[0, 1], 7.0);
    update(&mut m, &[1, 0], -3.5);
    println!("{m:?}");
    let mut cell = Matrix::Cell(2.0);
    grow(&mut cell);
    println!("{cell:?}");
    let mut shape = Shape::Points(vec![1.0, 2.0]);
    rename(&mut shape);
    println!("{shape:?}");
}
