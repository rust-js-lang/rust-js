// A constant of a type no other crate can make, its fields private, used in
// another module: rustc's value of it, branded, not its initializer, which
// would read its module's private items there, as chrono's
// `NaiveDate::AFTER_MAX` reads `MAX_YEAR`; and a namespace's own constant
// made by its function, as bitflags' `Perms::READ`, its value, not a call
// of its namespace before it's made (#2403).
mod date {
    const MAX_YEAR: i32 = 5;

    pub struct NaiveDate {
        yof: i32,
    }

    impl NaiveDate {
        /// One day after the maximum.
        pub(crate) const AFTER_MAX: NaiveDate = NaiveDate::from_yof((MAX_YEAR + 1) << 13);
        pub const READ: NaiveDate = NaiveDate::from_yof(1);

        pub(crate) const fn from_yof(yof: i32) -> NaiveDate {
            NaiveDate { yof }
        }

        pub fn succ_opt(&self) -> Option<NaiveDate> {
            if self.yof >> 13 >= MAX_YEAR {
                None
            } else {
                Some(NaiveDate { yof: self.yof + (1 << 13) })
            }
        }

        pub fn year(&self) -> i32 {
            self.yof >> 13
        }

        pub fn yof(&self) -> i32 {
            self.yof
        }
    }
}

mod datetime {
    use crate::date::NaiveDate;

    pub fn next_year(d: &NaiveDate) -> i32 {
        d.succ_opt().unwrap_or(NaiveDate::AFTER_MAX).year()
    }
}

fn main() {
    let last = date::NaiveDate::AFTER_MAX;
    println!("{}", datetime::next_year(&date::NaiveDate::READ));
    println!("{}", datetime::next_year(&last));
    println!("{}", date::NaiveDate::READ.yof());
}
