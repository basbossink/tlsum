use lib::{format_date_time, format_time, hours_mins, now, summarize_file, timelog_path};

const UNDEFINED_CHAR_REPRESENTATION: char = '\u{22a5}';

#[allow(clippy::print_stdout)]
fn main() -> anyhow::Result<()> {
    let time_log = timelog_path()?;
    let now = now()?;
    let summary = summarize_file(time_log, &now)?;
    let undefined = || Ok(format!("{}", UNDEFINED_CHAR_REPRESENTATION));
    println!(
        r"
{:─<71}
{:<45}{}
{:<45}{}
{:<45}{}
{:─<71}
{:─<71}
{:<45}{}
{:<45}{}
{:<45}{:<5} days 
{:─<71}
{:─<71}
{:<45}{}
{:<45}{}
{:<45}{}
{:<45}{}
{:<45}{}
{:<45}{}
{:─<71}",
        "─",
        "First punch in today:",
        format_time(summary.first_punchin_today)?,
        "Last punch in:",
        format_date_time(summary.last_punchin)?,
        "Last punch out:",
        format_date_time(summary.last_punchout)?,
        "─",
        "─",
        "Average number of hours worked per workday:",
        hours_mins(summary.avg_worked),
        "Total time worked:",
        hours_mins(summary.total_worked),
        "Number of days worked:",
        summary.num_days_worked,
        "─",
        "─",
        "Cummulative overtime per yesterday:",
        hours_mins(summary.overtime),
        "Worked today:",
        hours_mins(summary.worked_today),
        "Still to work (8hrs):",
        hours_mins(summary.still_to_work_8),
        "Still to work:",
        hours_mins(summary.still_to_work),
        "Time to leave (8hrs):",
        summary
            .time_to_leave_8
            .map_or_else(undefined, format_time)?,
        "Time to leave:",
        summary.time_to_leave.map_or_else(undefined, format_time)?,
        "─",
    );
    Ok(())
}
