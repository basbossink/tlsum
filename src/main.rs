use lib::{create_timestampformat, format_time, hours_mins, now, summarize_file, timelog_path};

fn main() -> anyhow::Result<()> {
    let time_log = timelog_path()?;
    let format = create_timestampformat();
    let now = now(&format)?;
    let summary = summarize_file(time_log, &now, &format)?;
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
