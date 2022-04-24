extern crate time;
use time::{PrimitiveDateTime, format_description as fd};
use time::error::Parse;

/// This is the default timestamp format used by Emacs.
static TIMESTAMP_FORMAT: &str = "[year]/[month]/[day] [hour repr:24]:[minute]:[second]";

/// The default file path Emacs uses to record timeclock-in|out records.
// static DEFAULT_TIMELOG_PATH: &str = ".emacs.d/.local/etc/timelog";

fn main() {
    println!("Hello, world!");
}

fn parse_timestamp(date_time: &str) -> Result<PrimitiveDateTime, Parse> {
    PrimitiveDateTime::parse(date_time, &fd::parse(TIMESTAMP_FORMAT).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate time;
    use time::Month;

    #[test]
    fn should_parse_to_primitive_datetime() {
        let result = parse_timestamp("2022/04/22 21:33:23").unwrap();
        assert_eq!(2022, result.year());
        assert_eq!(Month::April, result.month());
        assert_eq!(22, result.day());
        assert_eq!((21,33,23), result.as_hms());
    }
}