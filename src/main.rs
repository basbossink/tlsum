extern crate time;
use std::str::FromStr;
use time::error::Parse;
use time::macros::datetime;
use time::{format_description as fd, PrimitiveDateTime};

/// This is the default timestamp format used by Emacs.
static TIMESTAMP_FORMAT: &str = "[year]/[month]/[day] [hour repr:24]:[minute]:[second]";
static SPACE: char = ' ';
// static COMMENT: char = '#';

/// The default file path Emacs uses to record timeclock-in|out records.
// static DEFAULT_TIMELOG_PATH: &str = ".emacs.d/.local/etc/timelog";
#[derive(Debug, PartialEq)]

enum ClockType {
    In,
    Out,
}

impl FromStr for ClockType {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.chars().next() {
            None => Err(ParseError::EmptyClockType),
            Some('i') => Ok(ClockType::In),
            Some('o') => Ok(ClockType::Out),
            _ => Err(ParseError::UnknownClockType),
        }
    }
}

#[derive(Debug, Clone)]
enum ParseError {
    EmptyClockType,
    UnknownClockType,
    UnparseableDate(Option<Parse>),
}

#[derive(Debug)]
struct Entry<'a> {
    clock_type: ClockType,
    date_time: PrimitiveDateTime,
    project: Option<&'a str>,
}

fn find_from<'a>(s: &'a str, index: Option<usize>, pat: char) -> Option<usize> {
    index.map_or(None, |i| s[i..].find(pat).map_or(None, |j| Some(i + j)))
}

fn parse_line<'a>(s: &'a str) -> Result<Entry<'a>, ParseError> {
    let clock_type: ClockType = s[0..1].parse()?;
    let date_time_onward = &s[2..];
    let time_start_index = date_time_onward.find(SPACE).map(|t| t + 1);
    let date_time_end = find_from(&date_time_onward, time_start_index, SPACE)
        .ok_or(ParseError::UnparseableDate(None))?;
    let date_time_slice = &date_time_onward[0..date_time_end];
    let date_time =
        parse_timestamp(date_time_slice).map_err(|e| ParseError::UnparseableDate(Some(e)))?;
    let rest = &date_time_onward[date_time_end + 1..];
    let project = match rest.len() {
        0 => None,
        _ => Some(rest),
    };
    Ok(Entry {
        clock_type,
        date_time,
        project,
    })
}

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
        assert_eq!((21, 33, 23), result.as_hms());
    }

    #[test]
    fn should_parse_line() {
        let line = "i 2022/04/22 21:33:23 e:fc:fred";
        let result: Entry = parse_line(line).unwrap();
        assert_eq!(ClockType::In, result.clock_type);
        assert_eq!(Some("e:fc:fred"), result.project);
        assert_eq!(datetime!(2022 - 04 - 22 21:33:23), result.date_time);
    }
}
