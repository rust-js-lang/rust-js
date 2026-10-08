//! JS's [`Date`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date):
//! a moment, as milliseconds since 1970 UTC, read and set in the local time
//! zone or in UTC (ADR 0283). Its numbers are `f64`s, as JS's are: an
//! invalid date's are `NaN`, which no integer holds.

// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

use super::{JsError, JsObject};
use core::marker::PhantomData;

/// [`Date`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date):
/// `date::now()` is the time, `date::new()` a `Date` of it.
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
    pub fn set_date(&self, value: f64) -> f64 {
        unreachable!()
    }

    /// [`date.setFullYear()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setFullYear): sets its year, in the local time zone; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setFullYear")]
    pub fn set_full_year(&self, value: f64) -> f64 {
        unreachable!()
    }

    /// [`date.setHours()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setHours): sets its hour, 0 to 23, in the local time zone; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setHours")]
    pub fn set_hours(&self, value: f64) -> f64 {
        unreachable!()
    }

    /// [`date.setMilliseconds()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setMilliseconds): sets its millisecond, 0 to 999, in the local time zone; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setMilliseconds")]
    pub fn set_milliseconds(&self, value: f64) -> f64 {
        unreachable!()
    }

    /// [`date.setMinutes()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setMinutes): sets its minute, 0 to 59, in the local time zone; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setMinutes")]
    pub fn set_minutes(&self, value: f64) -> f64 {
        unreachable!()
    }

    /// [`date.setMonth()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setMonth): sets its month, 0 for January, in the local time zone; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setMonth")]
    pub fn set_month(&self, value: f64) -> f64 {
        unreachable!()
    }

    /// [`date.setSeconds()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setSeconds): sets its second, 0 to 59, in the local time zone; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setSeconds")]
    pub fn set_seconds(&self, value: f64) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCDate()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCDate): sets its day of the month, 1 to 31, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCDate")]
    pub fn set_utc_date(&self, value: f64) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCFullYear()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCFullYear): sets its year, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCFullYear")]
    pub fn set_utc_full_year(&self, value: f64) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCHours()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCHours): sets its hour, 0 to 23, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCHours")]
    pub fn set_utc_hours(&self, value: f64) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCMilliseconds()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCMilliseconds): sets its millisecond, 0 to 999, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCMilliseconds")]
    pub fn set_utc_milliseconds(&self, value: f64) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCMinutes()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCMinutes): sets its minute, 0 to 59, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCMinutes")]
    pub fn set_utc_minutes(&self, value: f64) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCMonth()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCMonth): sets its month, 0 for January, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCMonth")]
    pub fn set_utc_month(&self, value: f64) -> f64 {
        unreachable!()
    }

    /// [`date.setUTCSeconds()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/setUTCSeconds): sets its second, 0 to 59, in UTC; its new time.
    #[cfg_attr(rust_js, rust_js::link_name = "setUTCSeconds")]
    pub fn set_utc_seconds(&self, value: f64) -> f64 {
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

    /// [`date.toLocaleString()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/toLocaleString): its date and time as the user's locale writes them.
    #[cfg_attr(rust_js, rust_js::link_name = "toLocaleString")]
    pub fn to_locale_string(&self) -> String {
        unreachable!()
    }

    /// [`date.toLocaleDateString()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/toLocaleDateString): its date as the user's locale writes it.
    #[cfg_attr(rust_js, rust_js::link_name = "toLocaleDateString")]
    pub fn to_locale_date_string(&self) -> String {
        unreachable!()
    }

    /// [`date.toLocaleTimeString()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/toLocaleTimeString): its time as the user's locale writes it.
    #[cfg_attr(rust_js, rust_js::link_name = "toLocaleTimeString")]
    pub fn to_locale_time_string(&self) -> String {
        unreachable!()
    }
}

unsafe extern "Rust" {
    /// [`new Date()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/Date): now.
    #[link_name = "new Date"]
    pub safe fn new() -> &'static Date;

    /// [`new Date(time)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/Date): of milliseconds since 1970 UTC.
    #[link_name = "new Date"]
    pub safe fn new_with_time(time: f64) -> &'static Date;

    /// [`new Date(text)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/Date): of text `Date.parse` reads, an invalid date of
    /// other text.
    #[link_name = "new Date"]
    pub safe fn new_with_text(text: &str) -> &'static Date;

    /// [`new Date(year, month, day, hours, minutes, seconds, ms)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/Date):
    /// of its parts in the local time zone, the month 0 for January.
    #[link_name = "new Date"]
    pub safe fn new_with_parts(year: f64, month: f64, day: f64, hours: f64, minutes: f64, seconds: f64, ms: f64) -> &'static Date;

    /// [`Date.now()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/now): milliseconds since 1970 UTC.
    #[link_name = "Date.now"]
    pub safe fn now() -> f64;

    /// [`Date.parse(text)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/parse): the time text says, `NaN` of text it can't read.
    #[link_name = "Date.parse"]
    pub safe fn parse(text: &str) -> f64;

    /// [`Date.UTC(year, month, day, hours, minutes, seconds, ms)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Date/UTC):
    /// the time of these parts in UTC.
    #[link_name = "Date.UTC"]
    pub safe fn utc(year: f64, month: f64, day: f64, hours: f64, minutes: f64, seconds: f64, ms: f64) -> f64;
}
