//! JS's [`Date`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date):
//! a moment, as milliseconds since 1970 UTC, read and set in the local time
//! zone or in UTC (ADR 0283). What it gives is an `f64`, as an invalid
//! date's parts are `NaN`, which no integer holds; what it's given, a part,
//! an `i32`, as ReScript's Stdlib has them.

// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

use super::intl::DateTimeFormatOptions;
use super::{JsError, JsObject};
use core::marker::PhantomData;

/// [`Date`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date):
/// `date::now()` is the time, `date::new(None)` a `Date` of it.
#[cfg_attr(rust_js, rust_js::types = "Date")]
pub struct Date(PhantomData<JsObject>);

// An object, never `undefined`.
unsafe impl super::Defined for Date {}

impl Date {
    /// [`date.getDate()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getDate): the day of the month, 1 to 31, in the local time zone.
    #[cfg_attr(rust_js, rust_js::link_name = "getDate")]
    pub fn get_date(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getDay()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getDay): the day of the week, 0 for Sunday, in the local time zone.
    #[cfg_attr(rust_js, rust_js::link_name = "getDay")]
    pub fn get_day(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getFullYear()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getFullYear): the year, in the local time zone.
    #[cfg_attr(rust_js, rust_js::link_name = "getFullYear")]
    pub fn get_full_year(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getHours()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getHours): the hour, 0 to 23, in the local time zone.
    #[cfg_attr(rust_js, rust_js::link_name = "getHours")]
    pub fn get_hours(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getMilliseconds()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getMilliseconds): the millisecond, 0 to 999, in the local time zone.
    #[cfg_attr(rust_js, rust_js::link_name = "getMilliseconds")]
    pub fn get_milliseconds(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getMinutes()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getMinutes): the minute, 0 to 59, in the local time zone.
    #[cfg_attr(rust_js, rust_js::link_name = "getMinutes")]
    pub fn get_minutes(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getMonth()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getMonth): the month, 0 for January, in the local time zone.
    #[cfg_attr(rust_js, rust_js::link_name = "getMonth")]
    pub fn get_month(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getSeconds()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getSeconds): the second, 0 to 59, in the local time zone.
    #[cfg_attr(rust_js, rust_js::link_name = "getSeconds")]
    pub fn get_seconds(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getUTCDate()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getUTCDate): the day of the month, 1 to 31, in UTC.
    #[cfg_attr(rust_js, rust_js::link_name = "getUTCDate")]
    pub fn get_utc_date(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getUTCDay()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getUTCDay): the day of the week, 0 for Sunday, in UTC.
    #[cfg_attr(rust_js, rust_js::link_name = "getUTCDay")]
    pub fn get_utc_day(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getUTCFullYear()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getUTCFullYear): the year, in UTC.
    #[cfg_attr(rust_js, rust_js::link_name = "getUTCFullYear")]
    pub fn get_utc_full_year(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getUTCHours()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getUTCHours): the hour, 0 to 23, in UTC.
    #[cfg_attr(rust_js, rust_js::link_name = "getUTCHours")]
    pub fn get_utc_hours(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getUTCMilliseconds()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getUTCMilliseconds): the millisecond, 0 to 999, in UTC.
    #[cfg_attr(rust_js, rust_js::link_name = "getUTCMilliseconds")]
    pub fn get_utc_milliseconds(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getUTCMinutes()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getUTCMinutes): the minute, 0 to 59, in UTC.
    #[cfg_attr(rust_js, rust_js::link_name = "getUTCMinutes")]
    pub fn get_utc_minutes(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getUTCMonth()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getUTCMonth): the month, 0 for January, in UTC.
    #[cfg_attr(rust_js, rust_js::link_name = "getUTCMonth")]
    pub fn get_utc_month(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getUTCSeconds()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getUTCSeconds): the second, 0 to 59, in UTC.
    #[cfg_attr(rust_js, rust_js::link_name = "getUTCSeconds")]
    pub fn get_utc_seconds(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getTime()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getTime): milliseconds since 1970 UTC, `NaN` of an invalid date.
    #[cfg_attr(rust_js, rust_js::link_name = "getTime")]
    pub fn get_time(&self) -> f64 {
        unreachable!()
    }

    /// [`date.getTimezoneOffset()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/getTimezoneOffset): minutes from local time to UTC.
    #[cfg_attr(rust_js, rust_js::link_name = "getTimezoneOffset")]
    pub fn get_timezone_offset(&self) -> f64 {
        unreachable!()
    }

    /// [`date.valueOf()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/valueOf): its time, as `get_time`.
    #[cfg_attr(rust_js, rust_js::link_name = "valueOf")]
    pub fn value_of(&self) -> f64 {
        unreachable!()
    }

    /// [`date.setDate()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setDate): sets its day of the month, 1 to 31, in the local time zone; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setDate")]
    pub fn set_date(&self, value: i32) -> f64 {
        unreachable!()
    }

    /// [`date.setFullYear(year, month, day)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setFullYear): sets its year, and its month and day where they're given, in the local time zone; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setFullYear")]
    pub fn set_full_year(&self, year: i32, month: Option<i32>, day: Option<i32>) -> f64 {
        unreachable!()
    }

    /// [`date.setHours(hours, minutes, seconds, ms)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setHours): sets its hour, 0 to 23, and its minute, second and millisecond where they're given, in the local time zone; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setHours")]
    pub fn set_hours(&self, hours: i32, minutes: Option<i32>, seconds: Option<i32>, ms: Option<i32>) -> f64 {
        unreachable!()
    }

    /// [`date.setMilliseconds()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setMilliseconds): sets its millisecond, 0 to 999, in the local time zone; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setMilliseconds")]
    pub fn set_milliseconds(&self, value: i32) -> f64 {
        unreachable!()
    }

    /// [`date.setMinutes(minutes, seconds, ms)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setMinutes): sets its minute, 0 to 59, and its second and millisecond where they're given, in the local time zone; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setMinutes")]
    pub fn set_minutes(&self, minutes: i32, seconds: Option<i32>, ms: Option<i32>) -> f64 {
        unreachable!()
    }

    /// [`date.setMonth(month, day)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setMonth): sets its month, 0 for January, and its day of the month where it's given, in the local time zone; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setMonth")]
    pub fn set_month(&self, month: i32, day: Option<i32>) -> f64 {
        unreachable!()
    }

    /// [`date.setSeconds(seconds, ms)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setSeconds): sets its second, 0 to 59, and its millisecond where it's given, in the local time zone; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setSeconds")]
    pub fn set_seconds(&self, seconds: i32, ms: Option<i32>) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCDate()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCDate): sets its day of the month, 1 to 31, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCDate")]
    pub fn set_utc_date(&self, value: i32) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCFullYear(year, month, day)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCFullYear): sets its year, and its month and day where they're given, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCFullYear")]
    pub fn set_utc_full_year(&self, year: i32, month: Option<i32>, day: Option<i32>) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCHours(hours, minutes, seconds, ms)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCHours): sets its hour, 0 to 23, and its minute, second and millisecond where they're given, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCHours")]
    pub fn set_utc_hours(&self, hours: i32, minutes: Option<i32>, seconds: Option<i32>, ms: Option<i32>) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCMilliseconds()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCMilliseconds): sets its millisecond, 0 to 999, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCMilliseconds")]
    pub fn set_utc_milliseconds(&self, value: i32) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCMinutes(minutes, seconds, ms)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCMinutes): sets its minute, 0 to 59, and its second and millisecond where they're given, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCMinutes")]
    pub fn set_utc_minutes(&self, minutes: i32, seconds: Option<i32>, ms: Option<i32>) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCMonth(month, day)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCMonth): sets its month, 0 for January, and its day of the month where it's given, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCMonth")]
    pub fn set_utc_month(&self, month: i32, day: Option<i32>) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCSeconds(seconds, ms)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCSeconds): sets its second, 0 to 59, and its millisecond where it's given, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCSeconds")]
    pub fn set_utc_seconds(&self, seconds: i32, ms: Option<i32>) -> f64 {
        unreachable!()
    }

    /// [`date.setTime()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setTime): sets its time, milliseconds since 1970 UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setTime")]
    pub fn set_time(&self, time: f64) -> f64 {
        unreachable!()
    }

    /// [`date.toISOString()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/toISOString): `2026-10-08T12:00:00.000Z`, or what it throws of an invalid date, a `RangeError`.
    #[cfg_attr(rust_js, rust_js::link_name = "toISOString")]
    pub fn to_iso_string(&self) -> Result<String, &'static JsError> {
        unreachable!()
    }

    /// [`date.toJSON()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/toJSON): its ISO text, `None` of an invalid date.
    #[cfg_attr(rust_js, rust_js::link_name = "toJSON")]
    pub fn to_json(&self) -> Option<String> {
        unreachable!()
    }

    /// [`date.toString()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/toString): its date and time, as JS shows them.
    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string(&self) -> String {
        unreachable!()
    }

    /// [`date.toDateString()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/toDateString): its date, as JS shows it.
    #[cfg_attr(rust_js, rust_js::link_name = "toDateString")]
    pub fn to_date_string(&self) -> String {
        unreachable!()
    }

    /// [`date.toTimeString()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/toTimeString): its time, as JS shows it.
    #[cfg_attr(rust_js, rust_js::link_name = "toTimeString")]
    pub fn to_time_string(&self) -> String {
        unreachable!()
    }

    /// [`date.toUTCString()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/toUTCString): its date and time in UTC, as an HTTP header has them.
    #[cfg_attr(rust_js, rust_js::link_name = "toUTCString")]
    pub fn to_utc_string(&self) -> String {
        unreachable!()
    }

    /// [`date.toLocaleString(locales, options)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/toLocaleString): its date and time as `intl::date_time_format::new(locales, options)` writes it, the user's locale's where they aren't given.
    #[cfg_attr(rust_js, rust_js::link_name = "toLocaleString")]
    pub fn to_locale_string(&self, locales: Option<&[&str]>, options: Option<&DateTimeFormatOptions<'_>>) -> String {
        unreachable!()
    }

    /// [`date.toLocaleDateString(locales, options)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/toLocaleDateString): its date as `intl::date_time_format::new(locales, options)` writes it, the user's locale's where they aren't given.
    #[cfg_attr(rust_js, rust_js::link_name = "toLocaleDateString")]
    pub fn to_locale_date_string(&self, locales: Option<&[&str]>, options: Option<&DateTimeFormatOptions<'_>>) -> String {
        unreachable!()
    }

    /// [`date.toLocaleTimeString(locales, options)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/toLocaleTimeString): its time as `intl::date_time_format::new(locales, options)` writes it, the user's locale's where they aren't given.
    #[cfg_attr(rust_js, rust_js::link_name = "toLocaleTimeString")]
    pub fn to_locale_time_string(&self, locales: Option<&[&str]>, options: Option<&DateTimeFormatOptions<'_>>) -> String {
        unreachable!()
    }
}

/// [`new Date(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/Date):
/// now where there's no `value`; of milliseconds since 1970 UTC,
/// `Some(&0.0)`; of text `Date.parse` reads, `Some(&"2024-01-01")`, an
/// invalid date of other text; or a copy of a `Date`.
#[cfg_attr(rust_js, rust_js::link_name = "new Date")]
pub fn new(value: Option<&dyn IntoNumberOrStrOrDate>) -> &'static Date {
    unreachable!()
}

/// What a `number | string | Date` parameter takes: each as it is (ADR
/// 0229); an optional one, `Option<&dyn IntoNumberOrStrOrDate>`, takes
/// `Some(&value)` (ADR 0443).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `number | string | Date`",
    note = "an optional one, `Option<&dyn IntoNumberOrStrOrDate>`, takes `Some(&value)`"
)]
#[cfg_attr(rust_js, rust_js::types = "number | string | Date")]
pub trait IntoNumberOrStrOrDate: sealed::Sealed {}
impl IntoNumberOrStrOrDate for f64 {}
impl IntoNumberOrStrOrDate for &str {}
impl IntoNumberOrStrOrDate for &String {}
impl IntoNumberOrStrOrDate for &Date {}

mod sealed {
    pub trait Sealed {}
    impl Sealed for f64 {}
    impl Sealed for &str {}
    impl Sealed for &String {}
    impl Sealed for &super::Date {}
}

/// [`new Date(year, month, day, hours, minutes, seconds, ms)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/Date): of these parts in the local time
/// zone, the month 0 for January, the day 1 and the rest 0 where they aren't given.
#[cfg_attr(rust_js, rust_js::link_name = "new Date")]
pub fn new_with_year_and_month(year: i32, month: i32, day: Option<i32>, hours: Option<i32>, minutes: Option<i32>, seconds: Option<i32>, ms: Option<i32>) -> &'static Date {
    unreachable!()
}

/// [`Date.now()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/now): milliseconds since 1970 UTC.
#[cfg_attr(rust_js, rust_js::link_name = "Date.now")]
pub fn now() -> f64 {
    unreachable!()
}

/// [`Date.parse(text)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/parse): the time text says, `NaN` of text it can't read.
#[cfg_attr(rust_js, rust_js::link_name = "Date.parse")]
pub fn parse(text: &str) -> f64 {
    unreachable!()
}

/// [`Date.UTC(year, month, day, hours, minutes, seconds, ms)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/UTC): the time of these parts in UTC,
/// the month 0 and the day 1 and the rest 0 where they aren't given.
#[cfg_attr(rust_js, rust_js::link_name = "Date.UTC")]
pub fn utc(year: i32, month: Option<i32>, day: Option<i32>, hours: Option<i32>, minutes: Option<i32>, seconds: Option<i32>, ms: Option<i32>) -> f64 {
    unreachable!()
}
