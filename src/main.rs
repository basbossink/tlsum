extern crate time;
use anyhow::{anyhow, Context, Result};
use itertools::Itertools;
use std::env;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;
use time::error::Parse;
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

#[derive(Debug, Clone, Error)]
enum ParseError {
    #[error("unable to find clock in/out marker")]
    EmptyClockType,
    #[error("unknown clock type")]
    UnknownClockType,
    #[error("unable to parse date [{0:?}]")]
    UnparseableDate(Option<Parse>),
}

#[derive(Debug)]
struct Entry {
    clock_type: ClockType,
    date_time: PrimitiveDateTime,
}

#[derive(Debug)]
struct Backed {
    duration: Duration,
    start: PrimitiveDateTime,
}

impl Backed {
    fn new(punch_in: Entry, punch_out: Entry) -> Self {
        Backed {
            start: punch_in.date_time,
            duration: punch_out.date_time - punch_in.date_time,
        }
    }
}

struct Summary {
    num_days_worked: u32,
    first_punchin_today: PrimitiveDateTime,
    avg_worked: Duration,
    overtime: Duration,
    still_to_work_8: Duration,
    still_to_work: Duration,
    time_to_leave: PrimitiveDateTime,
    time_to_leave_8: PrimitiveDateTime,
    total_worked: Duration,
    worked_today: Duration,
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn timelog_path() -> Result<PathBuf> {
    let time_log = env::var_os(TIMELOG_ENV_VAR_NAME)
        .map(PathBuf::from)
        .unwrap_or(PathBuf::from(DEFAULT_TIMELOG_PATH));
    let err = format!("time log file [{:?}] does not exist", &time_log);
    time_log.exists().then(|| time_log).ok_or(anyhow!(err))
}

fn find_from<'a>(s: &'a str, index: Option<usize>, pat: char) -> Option<usize> {
    index.map_or(None, |i| s[i..].find(pat).map_or(None, |j| Some(i + j)))
}

fn parse_line(s: String) -> Result<Entry, ParseError> {
    let clock_type: ClockType = s[0..1].parse()?;
    let date_time_onward = &s[2..];
    let time_start_index = date_time_onward.find(SPACE).map(|t| t + 1);
    let date_time_end =
        find_from(&date_time_onward, time_start_index, SPACE).unwrap_or(date_time_onward.len());
    let date_time_slice = &date_time_onward[0..date_time_end];
    let date_time =
        parse_timestamp(date_time_slice).map_err(|e| ParseError::UnparseableDate(Some(e)))?;
    Ok(Entry {
        clock_type,
        date_time,
    })
}

enum States {
    Initial,
    ExpectingClockIn,
    ExpectingClockOut,
}

fn main() -> anyhow::Result<()> {
    let time_log = timelog_path()?;
    let lines = read_lines(&time_log).with_context(|| format!("unable to read {:?}", &time_log))?;
    let mut state = States::Initial;
    let mut previous: Option<Entry> = None;
    let mut backed: Option<Backed> = None;
    let mut backed_values: Vec<Backed> = Vec::new();
    for (line_number, line) in lines.enumerate() {
        let ip = line.with_context(|| format!("failed to parse line {}", line_number))?;
        let mine = ip.clone();
        let trimmed = mine.trim().to_string();

        if trimmed.starts_with(COMMENT) {
            continue;
        }
        let entry =
            parse_line(trimmed).with_context(|| format!("failed to parse line {}", line_number))?;
        (state, previous, backed) = (match (state, previous, &entry.clock_type) {
            (States::Initial, None, ClockType::In) => {
                Ok((States::ExpectingClockOut, Some(entry), None))
            }
            (States::Initial, None, ClockType::Out) => Err(anyhow!(
                "unexpected, clock out on line {}, no previous clock in",
                line_number
            )),
            (States::Initial, _, _) => {
                Err(anyhow!("illegal state reached at line {}", line_number))
            }
            (States::ExpectingClockIn, _, ClockType::Out) => Err(anyhow!(
                "unexpected, clock out on line {}, expecting clock in",
                line_number
            )),
            (States::ExpectingClockIn, _, ClockType::In) => {
                Ok((States::ExpectingClockOut, Some(entry), None))
            }
            (States::ExpectingClockOut, _, ClockType::In) => Err(anyhow!(
                "unexpected, clock in on line {}, expecting clock out",
                line_number
            )),
            (States::ExpectingClockOut, None, _) => {
                Err(anyhow!("illegal state reached at line {}", line_number))
            }
            (States::ExpectingClockOut, Some(prev), ClockType::Out) => Ok((
                States::ExpectingClockIn,
                None,
                Some(Backed::new(prev, entry)),
            )),
        })?;
        if let Some(val) = backed {
            backed_values.push(val);
            backed = None;
        }
    }
    if let (States::ExpectingClockOut, Some(prev), None) = (state, previous, backed) {
        let now = now()?;
        backed_values.push(Backed::new(
            prev,
            Entry {
                clock_type: ClockType::Out,
                date_time: now,
            },
        ))
    }
    let summary = summarize(&backed_values)?;
    println!(
        r"{:<45}{}
{:<45}{}
{:<45}{}
{:<45}{}
{:<45}{}
{:<45}{}
{:<45}{}
{:<45}{}
{:<45}{}
{:<45}{}",
        "Average number of hours worked per workday:",
        hours_mins(summary.avg_worked),
        "Number of days worked:",
        summary.num_days_worked,
        "Total time worked:",
        hours_mins(summary.total_worked),
        "Cummulative overtime per yesterday:",
        hours_mins(summary.overtime),
        "First punch in today:",
        format_time(summary.first_punchin_today),
        "Worked today:",
        hours_mins(summary.worked_today),
        "Still to work (8hrs):",
        hours_mins(summary.still_to_work_8),
        "Still to work:",
        hours_mins(summary.still_to_work),
        "Time to leave (8hrs):",
        format_time(summary.time_to_leave_8),
        "Time to leave:",
        format_time(summary.time_to_leave),
    );
    Ok(())
}

fn format_time(date_time: PrimitiveDateTime) -> String {
    let format = fd::parse("[hour]:[minute]").unwrap();
    date_time.time().format(&format).unwrap()
}

fn hours_mins(duration: Duration) -> String {
    let hours = duration.whole_hours();
    format!(
        "{}{} hours, {} minutes",
        if duration.is_negative() { "-" } else { "" },
        i64::abs(hours),
        i64::abs((duration - Duration::hours(hours)).whole_minutes())
    )
}
fn now() -> anyhow::Result<PrimitiveDateTime> {
    let now: PrimitiveDateTime = parse_timestamp(
        &OffsetDateTime::now_local()?.format(&fd::parse(TIMESTAMP_FORMAT).unwrap())?,
    )?;
    Ok(now)
}
fn summarize(backed_values: &Vec<Backed>) -> anyhow::Result<Summary> {
    let mut num_days_worked: u32 = 0;
    let mut first_punchin_today: PrimitiveDateTime = PrimitiveDateTime::MIN;
    let now = now()?;
    let today = now.date();
    let mut worked_today = Duration::ZERO;
    for (key, group) in &backed_values.into_iter().group_by(|b| b.start.date()) {
        num_days_worked += 1;
        if key == today {
            let day = group.collect::<Vec<&Backed>>();
            first_punchin_today = day[0].start;
            worked_today = day.iter().fold(worked_today, |acc, x| acc + x.duration);
        }
    }
    let total_worked = backed_values
        .iter()
        .fold(Duration::ZERO, |acc, x| acc + x.duration);
    let avg_worked = total_worked / num_days_worked;
    let total_worked_until_prev = backed_values
        .iter()
        .filter(|x| x.start.date() < today)
        .fold(Duration::ZERO, |acc, x| acc + x.duration);

    let overtime = total_worked_until_prev - ((num_days_worked - 1u32) * 8u32 * Duration::HOUR);
    let still_to_work_8 = (8u32 * Duration::HOUR) - worked_today;
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

fn parse_timestamp(date_time: &str) -> Result<PrimitiveDateTime, Parse> {
    PrimitiveDateTime::parse(date_time, &fd::parse(TIMESTAMP_FORMAT).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate time;
    use time::macros::datetime;
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
    fn should_parse_clock_in_line() {
        let line = "i 2022/04/22 21:33:23 e:fc:fred";
        let result = parse_line(line.to_string()).unwrap();
        assert_eq!(ClockType::In, result.clock_type);
        assert_eq!(datetime!(2022 - 04 - 22 21:33:23), result.date_time);
    }

    #[test]
    fn should_parse_clock_out_line() {
        let line = "o 2022/04/22 21:33:33";
        let result = parse_line(line.to_string()).unwrap();
        assert_eq!(ClockType::Out, result.clock_type);
        assert_eq!(datetime!(2022 - 04 - 22 21:33:33), result.date_time);
    }
}
