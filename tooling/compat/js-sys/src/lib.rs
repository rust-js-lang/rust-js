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

/// JS's [`Uint8Array`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Uint8Array),
/// a JS value of one, owned, as js-sys's is, a clone the same array: what
/// getrandom 0.1's browser backend fills and reads (#1404).
#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct Uint8Array {
    obj: JsValue,
}

impl From<JsValue> for Uint8Array {
    fn from(obj: JsValue) -> Self {
        Uint8Array { obj }
    }
}

impl From<Uint8Array> for JsValue {
    fn from(value: Uint8Array) -> Self {
        value.obj
    }
}

impl AsRef<JsValue> for Uint8Array {
    fn as_ref(&self) -> &JsValue {
        &self.obj
    }
}

impl Deref for Uint8Array {
    type Target = JsValue;

    fn deref(&self) -> &JsValue {
        &self.obj
    }
}

impl Uint8Array {
    /// `new Uint8Array(length)`: `length` zeros.
    pub fn new_with_length(length: u32) -> Uint8Array {
        #[cfg_attr(rust_js, rust_js::link_name = "new Uint8Array")]
        fn made(length: u32) -> Option<&'static js::Unknown> {
            unreachable!()
        }
        Uint8Array::from(JsValue { __value: made(length) })
    }

    /// `this.subarray(begin, end)`: a view of its bytes `begin` to `end`.
    pub fn subarray(&self, begin: u32, end: u32) -> Uint8Array {
        #[cfg_attr(rust_js, rust_js::link_name = "subarray")]
        fn sub(this: Option<&'static js::Unknown>, begin: u32, end: u32) -> Option<&'static js::Unknown> {
            unreachable!()
        }
        Uint8Array::from(JsValue { __value: sub(self.obj.__value, begin, end) })
    }

    /// `this.length`.
    pub fn length(&self) -> u32 {
        #[cfg_attr(rust_js, rust_js::link_name = "get length")]
        fn length(this: Option<&'static js::Unknown>) -> u32 {
            unreachable!()
        }
        length(self.obj.__value)
    }

    /// Its bytes copied into `dst`, of its length, as js-sys's asserts.
    pub fn copy_to(&self, dst: &mut [u8]) {
        #[cfg_attr(rust_js, rust_js::link_name = "get []")]
        fn at(this: Option<&'static js::Unknown>, index: u32) -> u8 {
            unreachable!()
        }
        assert_eq!(self.length() as usize, dst.len());
        for (index, byte) in dst.iter_mut().enumerate() {
            *byte = at(self.obj.__value, index as u32);
        }
    }
}
