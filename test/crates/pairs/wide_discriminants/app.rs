//! A library's enums of a `repr` wider than 32 bits, hashed, compared,
//! shown, cast and told apart by their discriminants here (#1131).
use dep::{Endless, Huge, Vast, Wide};

pub fn main() {
    println!("{} {}", dep::digest(&Wide::Low), dep::digest(&Wide::High(2)));
    println!("{} {}", dep::digest(&Huge::Small), dep::digest(&Huge::Big));
    println!("{} {}", dep::digest(&Vast::Neg), dep::digest(&Vast::Pos(1)));
    println!("{} {}", dep::digest(&Endless::First), dep::digest(&Endless::Last));
    println!("{:?} {:?} {:?} {:?}", Wide::High(3), Huge::Big, Vast::Neg, Endless::Last);
    println!("{} {}", Huge::Small < Huge::Big, Endless::First.cmp(&Endless::Last) as i8);
    println!("{} {}", Huge::Big as u64, dep::huge_number(Huge::Small));
    println!("{} {}", Endless::Last as u128, Endless::First as u128);
    println!(
        "{} {} {}",
        dep::same_wide(&Wide::High(1), &Wide::High(2)),
        dep::same_vast(&Vast::Neg, &Vast::Pos(0)),
        std::mem::discriminant(&Huge::Big) == std::mem::discriminant(&Huge::Big)
    );
}
