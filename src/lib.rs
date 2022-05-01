use anyhow::{anyhow, bail, Context, Result};
use std::env;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;
use time::error::Parse;
use time::Date;
use time::Duration;
use time::OffsetDateTime;
use time::{format_description as fd, PrimitiveDateTime};

/// This is the default timestamp format used by Emacs.
const TIMESTAMP_FORMAT: &str = "[year]/[month]/[day] [hour repr:24]:[minute]:[second]";
const SPACE: char = ' ';
const TIMELOG_ENV_VAR_NAME: &str = "TIMELOG";
const COMMENT: char = '#';

/// The default file path Emacs uses to record timeclock-in|out records.
const DEFAULT_TIMELOG_PATH: &str = ".emacs.d/.local/etc/timelog";

#[derive(Debug, PartialEq, Copy, Clone)]
enum ClockType {
    In,
    Out,
}

impl FromStr for ClockType {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.chars().next() {
            None => Err(ParseError::EmptyClockType),
            Some('i' | 'I') => Ok(ClockType::In),
            Some('o' | 'O') => Ok(ClockType::Out),
            _ => Err(ParseError::UnknownClockType),
        }
    }
}

#[derive(Debug, Clone, Error)]
enum ParseError {
    #[error("unable to find clock in/out marker")]
    EmptyClockType,
    #[error("unknown clock type")]
    UnknownClockType,
    #[error("unable to parse date [{0:?}]")]
    UnparseableDate(Option<Parse>),
}

pub struct Summary {
    pub num_days_worked: u32,
    pub first_punchin_today: PrimitiveDateTime,
    pub avg_worked: Duration,
    pub overtime: Duration,
    pub still_to_work_8: Duration,
    pub still_to_work: Duration,
    pub time_to_leave: PrimitiveDateTime,
    pub time_to_leave_8: PrimitiveDateTime,
    pub total_worked: Duration,
    pub worked_today: Duration,
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

pub fn timelog_path() -> Result<PathBuf> {
    let time_log = env::var_os(TIMELOG_ENV_VAR_NAME)
        .map_or_else(|| PathBuf::from(DEFAULT_TIMELOG_PATH), PathBuf::from);
    let err = format!("time log file [{:?}] does not exist", &time_log);
    time_log
        .exists()
        .then(|| time_log)
        .ok_or_else(|| anyhow!(err))
}

fn find_from(s: &str, index: Option<usize>, pat: char) -> Option<usize> {
    index.and_then(|i| s[i..].find(pat).map(|j| i + j))
}

fn parse_line(
    s: &str,
    format: &Vec<fd::FormatItem<'_>>,
) -> Result<(ClockType, PrimitiveDateTime), ParseError> {
    let clock_type: ClockType = s[0..1].parse()?;
    let date_time_onward = &s[2..];
    let time_start_index = date_time_onward.find(SPACE).map(|t| t + 1);
    let date_time_end =
        find_from(date_time_onward, time_start_index, SPACE).unwrap_or(date_time_onward.len());
    let date_time_slice = &date_time_onward[0..date_time_end];
    let date_time = parse_timestamp(date_time_slice, format)
        .map_err(|e| ParseError::UnparseableDate(Some(e)))?;
    Ok((clock_type, date_time))
}

enum States {
    ExpectingClockIn,
    ExpectingClockOut,
}

pub fn summarize_lines<P>(filename: P) -> anyhow::Result<Summary>
where
    P: AsRef<Path>,
{
    let lines = read_lines(&filename)
        .with_context(|| format!("unable to read {}", filename.as_ref().to_string_lossy()))?;
    let format = create_timestampformat();
    let mut state = States::ExpectingClockIn;
    let mut clockin = PrimitiveDateTime::MIN;
    let mut worked_today: Duration = Duration::ZERO;
    let mut first_punchin_today: PrimitiveDateTime = PrimitiveDateTime::MIN;
    let mut total_worked: Duration = Duration::ZERO;
    let mut num_days_worked: u32 = 0;
    let mut line_number: usize = 0;
    let mut previous_date: Date = PrimitiveDateTime::MIN.date();
    for line in lines {
        line_number += 1;
        let ip = line.with_context(|| format!("failed to parse line {}", line_number))?;
        let trimmed = ip.trim().to_owned();
        if trimmed.starts_with(COMMENT) {
            continue;
        }

        let (clock_type, time_stamp) = parse_line(&trimmed, &format)
            .with_context(|| format!("failed to parse line {}", line_number))?;
        state = (match (state, clock_type) {
            (States::ExpectingClockIn, ClockType::In) => {
                let current_date = time_stamp.date();
                if previous_date != current_date {
                    worked_today = Duration::ZERO;
                    num_days_worked += 1;
                    first_punchin_today = time_stamp;
                    previous_date = current_date;
                }
                clockin = time_stamp;
                Ok(States::ExpectingClockOut)
            }
            (States::ExpectingClockOut, ClockType::Out) => {
                if time_stamp < clockin {
                    bail!(
                        "clock out time before clock in time on line {}",
                        line_number
                    );
                }
                let clocked = time_stamp - clockin;
                worked_today += clocked;
                total_worked += clocked;
                Ok(States::ExpectingClockIn)
            }
            (States::ExpectingClockIn, ClockType::Out) => Err(anyhow!(
                "unexpected, clock out on line {}, expecting clock in",
                line_number
            )),
            (States::ExpectingClockOut, ClockType::In) => Err(anyhow!(
                "unexpected, clock in on line {}, expecting clock out",
                line_number
            )),
        })?;
    }
    if let States::ExpectingClockOut = state {
        let now = now(&format)?;
        if now < clockin {
            bail!("now is before clock in time on line {}", line_number);
        }
        let clocked = now - clockin;
        worked_today += clocked;
        total_worked += clocked;
    }
    let summary = summarize(
        worked_today,
        first_punchin_today,
        total_worked,
        num_days_worked,
        &format,
    )?;
    Ok(summary)
}

#[must_use]
pub fn format_time(date_time: PrimitiveDateTime) -> String {
    let format = fd::parse("[hour]:[minute]").unwrap();
    date_time.time().format(&format).unwrap()
}

#[must_use]
pub fn hours_mins(duration: Duration) -> String {
    let hours = duration.whole_hours();
    format!(
        "{}{} hours, {} minutes",
        if duration.is_negative() { "-" } else { "" },
        i64::abs(hours),
        i64::abs((duration - Duration::hours(hours)).whole_minutes())
    )
}

fn now(format: &Vec<fd::FormatItem<'_>>) -> anyhow::Result<PrimitiveDateTime> {
    let now: PrimitiveDateTime =
        parse_timestamp(&OffsetDateTime::now_local()?.format(format)?, format)?;
    Ok(now)
}

fn summarize(
    worked_today: Duration,
    first_punchin_today: PrimitiveDateTime,
    total_worked: Duration,
    num_days_worked: u32,
    format: &Vec<fd::FormatItem<'_>>,
) -> anyhow::Result<Summary> {
    let now = now(format)?;
    let avg_worked = total_worked / num_days_worked;
    let total_worked_until_prev = total_worked - worked_today;
    let overtime = total_worked_until_prev - ((num_days_worked - 1_u32) * 8_u32 * Duration::HOUR);
    let still_to_work_8 = (8_u32 * Duration::HOUR) - worked_today;
    let still_to_work = still_to_work_8 - overtime;
    let time_to_leave = now + still_to_work;
    let time_to_leave_8 = now + still_to_work_8;
    Ok(Summary {
        num_days_worked,
        first_punchin_today,
        avg_worked,
        overtime,
        still_to_work_8,
        still_to_work,
        time_to_leave,
        time_to_leave_8,
        total_worked,
        worked_today,
    })
}

fn parse_timestamp(
    date_time: &str,
    format: &Vec<fd::FormatItem<'_>>,
) -> Result<PrimitiveDateTime, Parse> {
    PrimitiveDateTime::parse(date_time, format)
}

fn create_timestampformat() -> Vec<fd::FormatItem<'static>> {
    fd::parse(TIMESTAMP_FORMAT).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate time;
    use time::macros::datetime;
    use time::Month;

    #[test]
    fn should_parse_to_primitive_datetime() {
        let format = create_timestampformat();
        let result = parse_timestamp("2022/04/22 21:33:23", &format).unwrap();
        assert_eq!(2022, result.year());
        assert_eq!(Month::April, result.month());
        assert_eq!(22, result.day());
        assert_eq!((21, 33, 23), result.as_hms());
    }

    #[test]
    fn should_parse_clock_in_line() {
        let format = create_timestampformat();
        let line = "i 2022/04/22 21:33:23 e:fc:fred";
        let (clock_type, date_time) = parse_line(line, &format).unwrap();
        assert_eq!(ClockType::In, clock_type);
        assert_eq!(datetime!(2022 - 04 - 22 21:33:23), date_time);
    }

    #[test]
    fn should_parse_clock_out_line() {
        let format = create_timestampformat();
        let line = "o 2022/04/22 21:33:33";
        let (clock_type, date_time) = parse_line(line, &format).unwrap();
        assert_eq!(ClockType::Out, clock_type);
        assert_eq!(datetime!(2022 - 04 - 22 21:33:33), date_time);
    }
}
