use anyhow::{anyhow, bail, Context, Result};
use std::{
    env,
    fs::File,
    io,
    io::BufRead,
    ops::{Range, RangeTo},
    path::Path,
    path::PathBuf,
    str::FromStr,
};
use time::{
    error::Parse, format_description::FormatItem, macros::format_description, Date, Duration,
    OffsetDateTime, PrimitiveDateTime,
};

/// This is the default timestamp format used by Emacs.
const TIMESTAMP_FORMAT: &[FormatItem<'static>] =
    format_description!("[year]/[month]/[day] [hour repr:24]:[minute]:[second]");
const HOUR_MINUTE_FORMAT: &[FormatItem<'static>] = format_description!("[hour]:[minute]");

const TIMELOG_ENV_VAR_NAME: &str = "TIMELOG";
const COMMENT: char = '#';

/// The default file path Emacs uses to record timeclock-in|out records.
const DEFAULT_TIMELOG_PATH: &str = ".emacs.d/.local/etc/timelog";

//           1         2
// 012345678901234567890123456
// i 2022/04/22 21:33:23 e:fc:fred
const CLOCK_TYPE_RANGE: RangeTo<usize> = ..1;
const DATE_TIME_RANGE: Range<usize> = 2..21;

#[derive(Debug, PartialEq, Copy, Clone)]
enum ClockType {
    In,
    Out,
}

impl FromStr for ClockType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.chars().next() {
            None => Err(anyhow!("unable to find clock in/out marker")),
            Some('i' | 'I') => Ok(Self::In),
            Some('o' | 'O') => Ok(Self::Out),
            Some(other) => Err(anyhow!(format!("unknown clock type: [{}]", other))),
        }
    }
}

pub struct Summary {
    pub num_days_worked: u32,
    pub first_punchin_today: PrimitiveDateTime,
    pub avg_worked: Duration,
    pub overtime: Duration,
    pub still_to_work_8: Duration,
    pub still_to_work: Duration,
    pub time_to_leave: Option<PrimitiveDateTime>,
    pub time_to_leave_8: Option<PrimitiveDateTime>,
    pub total_worked: Duration,
    pub worked_today: Duration,
}

impl Summary {
    #[must_use]
    fn new(
        worked_today: Duration,
        first_punchin_today: PrimitiveDateTime,
        total_worked: Duration,
        num_days_worked: u32,
        now: &PrimitiveDateTime,
        clocked_in: bool,
    ) -> Self {
        let avg_worked = total_worked / num_days_worked;
        let total_worked_until_prev = total_worked - worked_today;
        let overtime =
            total_worked_until_prev - ((num_days_worked - 1_u32) * 8_u32 * Duration::HOUR);
        let still_to_work_8 = (8_u32 * Duration::HOUR) - worked_today;
        let still_to_work = still_to_work_8 - overtime;
        let time_to_leave = clocked_in.then(|| *now + still_to_work);
        let time_to_leave_8 = clocked_in.then(|| *now + still_to_work_8);
        Self {
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
        }
    }
}

#[inline]
pub fn timelog_path() -> Result<PathBuf> {
    let time_log = env::var_os(TIMELOG_ENV_VAR_NAME)
        .map_or_else(|| PathBuf::from(DEFAULT_TIMELOG_PATH), PathBuf::from);
    if time_log.exists() {
        Ok(time_log)
    } else {
        bail!("time log file [{:?}] does not exist", time_log)
    }
}

fn parse_line(s: &str) -> anyhow::Result<(ClockType, PrimitiveDateTime)> {
    let clock_type_slice = s
        .get(CLOCK_TYPE_RANGE)
        .ok_or_else(|| anyhow::anyhow!("got empty slice, expected 'i'| 'o'"))?;
    let clock_type: ClockType = clock_type_slice.parse()?;
    let date_time_slice = s
        .get(DATE_TIME_RANGE)
        .ok_or_else(|| anyhow::anyhow!(format!("expected slice with size 18")))?;
    let date_time = parse_timestamp(date_time_slice)
        .with_context(|| format!("unable to parse timestamp: [{}]", date_time_slice))?;
    Ok((clock_type, date_time))
}

#[derive(Debug, PartialEq, Eq)]
enum States {
    ExpectingClockIn,
    ExpectingClockOut,
}

fn summarize_lines(reader: Box<dyn BufRead>, now: &PrimitiveDateTime) -> anyhow::Result<Summary> {
    let lines = reader.lines();
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
        let ip = line.with_context(|| format!("failed to read line {}", line_number))?;
        if ip.starts_with(COMMENT) || ip.is_empty() {
            continue;
        }

        let (clock_type, time_stamp) =
            parse_line(&ip).with_context(|| format!("failed to parse line {}", line_number))?;
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
    let clocked_in = States::ExpectingClockOut == state;
    if clocked_in {
        if now < &clockin {
            bail!("now is before clock in time on line {}", line_number);
        }
        let clocked = *now - clockin;
        worked_today += clocked;
        total_worked += clocked;
    }
    let summary = Summary::new(
        worked_today,
        first_punchin_today,
        total_worked,
        num_days_worked,
        now,
        clocked_in,
    );
    Ok(summary)
}

#[inline]
pub fn summarize_file<P>(filename: P, now: &PrimitiveDateTime) -> anyhow::Result<Summary>
where
    P: AsRef<Path>,
{
    let file = File::open(&filename)
        .with_context(|| format!("unable to read {}", &filename.as_ref().to_string_lossy()))?;
    summarize_lines(
        Box::new(io::BufReader::with_capacity(512 * 1024, file)),
        now,
    )
}

#[inline]
pub fn format_time(date_time: PrimitiveDateTime) -> anyhow::Result<String> {
    date_time
        .time()
        .format(HOUR_MINUTE_FORMAT)
        .context("unable to format time")
}

#[must_use]
#[inline]
pub fn hours_mins(duration: Duration) -> String {
    let hours = duration.whole_hours();
    format!(
        "{}{} hours, {} minutes",
        if duration.is_negative() { "-" } else { "" },
        i64::abs(hours),
        i64::abs((duration - Duration::hours(hours)).whole_minutes())
    )
}

#[inline]
pub fn now() -> anyhow::Result<PrimitiveDateTime> {
    let now: PrimitiveDateTime =
        parse_timestamp(&OffsetDateTime::now_local()?.format(TIMESTAMP_FORMAT)?)?;
    Ok(now)
}

fn parse_timestamp(date_time: &str) -> Result<PrimitiveDateTime, Parse> {
    PrimitiveDateTime::parse(date_time, TIMESTAMP_FORMAT)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_timestamp {
        use super::*;
        use time::Month;

        #[test]
        fn should_parse_to_primitive_datetime() {
            let result = parse_timestamp("2022/04/22 21:33:23").unwrap();
            assert_eq!(2022, result.year());
            assert_eq!(Month::April, result.month());
            assert_eq!(22, result.day());
            assert_eq!((21, 33, 23), result.as_hms());
        }
    }

    mod parse_line {
        use super::*;
        use time::macros::datetime;

        #[test]
        fn should_parse_clock_in_line() {
            let line = "i 2022/04/22 21:33:23 e:fc:fred";
            let (clock_type, date_time) = parse_line(line).unwrap();
            assert_eq!(ClockType::In, clock_type);
            assert_eq!(datetime!(2022 - 04 - 22 21:33:23), date_time);
        }

        #[test]
        fn should_parse_clock_out_line() {
            let line = "o 2022/04/22 21:33:33";
            let (clock_type, date_time) = parse_line(line).unwrap();
            assert_eq!(ClockType::Out, clock_type);
            assert_eq!(datetime!(2022 - 04 - 22 21:33:33), date_time);
        }
    }

    mod summary {
        use super::*;
        use time::macros::datetime;

        struct SummaryNewTestCase {
            worked_today: Duration,
            first_punchin_today: PrimitiveDateTime,
            total_worked: Duration,
            num_days_worked: u32,
            now: PrimitiveDateTime,
            clocked_in: bool,
            expected_overtime: Duration,
            expected_avg_worked: Duration,
            expected_still_to_work: Duration,
            expected_still_to_work_8: Duration,
            expected_time_to_leave: Option<PrimitiveDateTime>,
            expected_time_to_leave_8: Option<PrimitiveDateTime>,
        }

        fn aaa_summary_new(tc: &SummaryNewTestCase) {
            let result = Summary::new(
                tc.worked_today,
                tc.first_punchin_today,
                tc.total_worked,
                tc.num_days_worked,
                &tc.now,
                tc.clocked_in,
            );
            assert_eq!(result.num_days_worked, tc.num_days_worked);
            assert_eq!(result.first_punchin_today, tc.first_punchin_today);
            assert_eq!(result.total_worked, tc.total_worked);
            assert_eq!(result.avg_worked, tc.expected_avg_worked);
            assert_eq!(result.overtime, tc.expected_overtime);
            assert_eq!(result.still_to_work, tc.expected_still_to_work);
            assert_eq!(result.still_to_work_8, tc.expected_still_to_work_8);
            assert_eq!(result.time_to_leave_8, tc.expected_time_to_leave_8);
            assert_eq!(result.time_to_leave, tc.expected_time_to_leave);
        }

        #[test]
        fn summary_new_today_only() {
            let tc = SummaryNewTestCase {
                now: datetime!(2022 - 04 - 22 09:33:33),
                worked_today: Duration::hours(3_i64),
                first_punchin_today: datetime!(2022 - 04 - 22 06:33:33),
                total_worked: Duration::hours(3_i64),
                num_days_worked: 1u32,
                clocked_in: true,
                expected_overtime: Duration::ZERO,
                expected_avg_worked: Duration::hours(3_i64),
                expected_still_to_work: Duration::hours(5_i64),
                expected_still_to_work_8: Duration::hours(5_i64),
                expected_time_to_leave: Some(datetime!(2022 - 04 - 22 14:33:33)),
                expected_time_to_leave_8: Some(datetime!(2022 - 04 - 22 14:33:33)),
            };
            aaa_summary_new(&tc);
        }

        #[test]
        fn summary_new_2_days_positive_overtime() {
            let tc = SummaryNewTestCase {
                now: datetime!(2022 - 04 - 22 09:33:33),
                worked_today: Duration::hours(3_i64),
                first_punchin_today: datetime!(2022 - 04 - 22 06:33:33),
                total_worked: Duration::hours(12_i64),
                num_days_worked: 2u32,
                clocked_in: true,
                expected_overtime: Duration::hours(1_i64),
                expected_avg_worked: Duration::hours(6_i64),
                expected_still_to_work: Duration::hours(4_i64),
                expected_still_to_work_8: Duration::hours(5_i64),
                expected_time_to_leave: Some(datetime!(2022 - 04 - 22 13:33:33)),
                expected_time_to_leave_8: Some(datetime!(2022 - 04 - 22 14:33:33)),
            };
            aaa_summary_new(&tc);
        }

        #[test]
        fn summary_new_2_days_negative_overtime() {
            let tc = SummaryNewTestCase {
                now: datetime!(2022 - 04 - 22 09:33:33),
                worked_today: Duration::hours(3_i64),
                first_punchin_today: datetime!(2022 - 04 - 22 06:33:33),
                total_worked: Duration::hours(8_i64),
                num_days_worked: 2u32,
                clocked_in: true,
                expected_overtime: Duration::hours(-3_i64),
                expected_avg_worked: Duration::hours(4_i64),
                expected_still_to_work: Duration::hours(8_i64),
                expected_still_to_work_8: Duration::hours(5_i64),
                expected_time_to_leave: Some(datetime!(2022 - 04 - 22 17:33:33)),
                expected_time_to_leave_8: Some(datetime!(2022 - 04 - 22 14:33:33)),
            };
            aaa_summary_new(&tc);
        }

        #[test]
        fn summary_new_last_state_is_clocked_out() {
            let tc = SummaryNewTestCase {
                now: datetime!(2022 - 04 - 22 09:33:33),
                worked_today: Duration::hours(3_i64),
                first_punchin_today: datetime!(2022 - 04 - 22 06:33:33),
                total_worked: Duration::hours(8_i64),
                num_days_worked: 2u32,
                clocked_in: false,
                expected_overtime: Duration::hours(-3_i64),
                expected_avg_worked: Duration::hours(4_i64),
                expected_still_to_work: Duration::hours(8_i64),
                expected_still_to_work_8: Duration::hours(5_i64),
                expected_time_to_leave: None,
                expected_time_to_leave_8: None,
            };
            aaa_summary_new(&tc);
        }
    }

    mod summarize_lines {
        use super::summarize_lines as sut;
        use super::*;
        use std::io::{BufReader, Cursor};
        use time::macros::datetime;

        fn create_reader(s: &'static str) -> Box<dyn BufRead> {
            let buff = Cursor::new(s);
            let reader = BufReader::new(buff);
            Box::new(reader)
        }

        #[test]
        fn account_for_still_clocked_in() {
            let content = "i 2022/01/01 09:00:00 fred:flintstone";
            let now = datetime!(2022 - 01 - 01 12:00:00);
            let reader = create_reader(content);
            let result = sut(reader, &now).unwrap();
            assert_eq!(result.total_worked, Duration::hours(3i64));
            assert_eq!(result.worked_today, Duration::hours(3i64));
        }

        #[test]
        fn account_for_not_clocked_in() {
            let content = r"i 2022/01/01 09:00:00 fred:flintstone
o 2022/01/01 11:00:00";
            let now = datetime!(2022 - 01 - 01 12:00:00);
            let reader = create_reader(content);
            let result = sut(reader, &now).unwrap();
            assert_eq!(result.total_worked, Duration::hours(2i64));
            assert_eq!(result.worked_today, Duration::hours(2i64));
            assert_eq!(result.time_to_leave, None);
            assert_eq!(result.time_to_leave_8, None);
        }
    }
}
