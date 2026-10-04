// `is_normal()`, `is_subnormal()` and `classify()` of an `f32` and an `f64`,
// as num-traits' `Float` asks: by each type's smallest normal value.

use std::num::FpCategory;

fn main() {
    let f32s = [1.0f32, 0.0, -0.0, f32::MIN_POSITIVE, 1e-40, -1e-45, f32::NAN, f32::INFINITY, f32::MAX];
    for x in f32s {
        print!("{}{} ", x.is_normal() as u8, x.is_subnormal() as u8);
    }
    println!();
    let f64s = [1.0f64, 0.0, f64::MIN_POSITIVE, 1e-310, 5e-324, 1e-40, f64::NAN, f64::NEG_INFINITY];
    for x in f64s {
        print!("{}{} ", x.is_normal() as u8, x.is_subnormal() as u8);
    }
    println!();
    for x in [1.0f32, 0.0, 1e-40, f32::NAN, f32::NEG_INFINITY] {
        let kind = match x.classify() {
            FpCategory::Normal => "n",
            FpCategory::Subnormal => "s",
            FpCategory::Zero => "z",
            FpCategory::Infinite => "i",
            FpCategory::Nan => "?",
        };
        print!("{} {:?} ", kind, x.classify());
    }
    println!("{}", 5e-324f64.classify() == FpCategory::Subnormal);
}
