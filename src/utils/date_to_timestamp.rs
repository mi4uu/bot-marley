use chrono::{
    format::ParseError, NaiveDate,
};


/// Converts a date string (YYYY-MM-DD) into a Unix timestamp (seconds since epoch).
///
/// The conversion assumes the time is midnight (00:00:00) and the timezone is UTC.
///
/// # Arguments
/// * `date_str`: A string slice representing the date (e.g., "2025-08-12").
///
/// # Returns
/// A `Result` containing the Unix timestamp as an `i64` on success, or a `ParseError` on failure.
pub fn date_string_to_timestamp(date_str: &str) -> Result<i64, ParseError> {
    // 1. Parse the string into a NaiveDate using the expected format "%Y-%m-%d"
    let naive_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;

    // 2. Combine the date with midnight time (00:00:00) to create a NaiveDateTime
    let naive_datetime = naive_date
        .and_hms_opt(0, 0, 0)
        // Unwrap is safe here as 00:00:00 is a valid time
        .unwrap();

    // 3. Attach the UTC timezone to make it a Zoned DateTime.
    let datetime_utc = naive_datetime.and_utc();

    // 4. Convert the UTC DateTime to a Unix timestamp (i64 seconds)
    Ok(datetime_utc.timestamp())
}
