//! rust-js's js-sys: js-sys's API, as crates use it: JS's builtins, `Date`, `Uint8Array`, `global()`, for an app's dependencies only (ADR 1112).
//! An app's own code binds JS with the `js` and `webapi` crates.

use core::marker::PhantomData;
use core::ops::Deref;

use js::JsObject;
use wasm_bindgen::JsValue;

/// JS's [`Date`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date),
/// `js::Date`, which it derefs to, its methods `js::Date`'s own: the
/// constructors are js-sys's names of `new Date(..)`.
#[cfg_attr(rust_js, rust_js::types = "Date")]
pub struct Date(PhantomData<JsObject>);

// An object, never `undefined`.
unsafe impl js::Defined for Date {}

impl Deref for Date {
    type Target = js::Date;

    fn deref(&self) -> &js::Date {
        unsafe { &*(self as *const Self).cast::<js::Date>() }
    }
}

impl Date {
    /// `new Date()`: now.
    #[cfg_attr(rust_js, rust_js::link_name = "new Date")]
    pub fn new_0() -> Date {
        unreachable!()
    }

    /// `new Date(init)`: of milliseconds since 1970 UTC, text `Date.parse`
    /// reads, or another `Date`.
    #[cfg_attr(rust_js, rust_js::link_name = "new Date")]
    pub fn new(init: &JsValue) -> Date {
        unreachable!()
    }

    /// `new Date(year, month, day, hours, minutes, seconds)`, of these parts
    /// in the local time zone, the month 0 for January.
    #[cfg_attr(rust_js, rust_js::link_name = "new Date")]
    pub fn new_with_year_month_day_hr_min_sec(year: u32, month: i32, day: i32, hrs: i32, mins: i32, secs: i32) -> Date {
        unreachable!()
    }
}
