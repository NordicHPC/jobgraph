use chrono::{Duration, NaiveDateTime};

pub fn date_range(t1: i64, t2: i64) -> Vec<String> {
    let d1 = NaiveDateTime::from_timestamp_opt(t1, 0).expect("REASON");
    let d2 = NaiveDateTime::from_timestamp_opt(t2, 0).expect("REASON");

    let mut date_range = Vec::new();

    let mut current_date = d1;
    while current_date <= d2 {
        date_range.push(current_date.format("%Y-%m-%d").to_string());
        current_date += Duration::days(1);
    }

    date_range
}
