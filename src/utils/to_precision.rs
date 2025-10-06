/// Converts a u64 (assumed to be scaled by 10^precision) into a String
/// representation, rounded to the given number of decimal places.
///
/// # Arguments
///
/// * `value` - The scaled u64 integer (e.g., 123456 for 1234.56).
/// * `precision` - The number of decimal places to round/display (e.g., 2).
///
/// # Examples
///
/// ```
/// use botmarley::utils::to_precision::round_u64_to_precision;
/// assert_eq!(round_u64_to_precision(123.45678, 2), "123.46");
/// assert_eq!(round_u64_to_precision(123.45478, 2), "123.45");

///
/// assert_eq!(round_u64_to_precision(123.456789, 5), "123.45679");
///
/// assert_eq!(round_u64_to_precision(123.45678, 0), "123");
/// ```
pub fn round_u64_to_precision(value: f64, precision: usize) -> String {
    format!("{value:.precision$}", value=value, precision=precision  )
}
