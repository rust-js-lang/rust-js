use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};

pub fn report() -> String {
    let mut lines = Vec::new();
    let date = NaiveDate::from_ymd_opt(2024, 2, 28).unwrap();
    for days in [0, 1, 2, 366, -60] {
        let d = date + Duration::days(days);
        lines.push(format!(
            "{d} {} {:?} ordinal {} week {} leap {}",
            d.format("%Y/%m/%d %a"),
            d.weekday(),
            d.ordinal(),
            d.iso_week().week(),
            d.leap_year(),
        ));
    }
    lines.push(format!("{:?}", NaiveDate::from_ymd_opt(2023, 2, 29)));
    let time = NaiveTime::from_hms_milli_opt(23, 59, 30, 250).unwrap();
    let at = NaiveDateTime::new(date, time);
    lines.push(format!("{at} {} {}", at + Duration::seconds(45), at.hour()));
    lines.push(format!(
        "{:?}",
        NaiveDateTime::parse_from_str("2024-03-01 12:30:00", "%Y-%m-%d %H:%M:%S")
    ));
    lines.push(format!("{:?}", NaiveDate::parse_from_str("31/12/1999", "%d/%m/%Y")));
    lines.push(format!(
        "{}",
        (NaiveDate::from_ymd_opt(2025, 1, 1).unwrap() - date).num_days()
    ));
    lines.push(format!("{:?}", Weekday::Sun.succ()));
    let mut dates = vec![
        date,
        NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
        date.pred_opt().unwrap(),
    ];
    dates.sort();
    lines.push(format!("{dates:?}"));
    lines.join("\n")
}
