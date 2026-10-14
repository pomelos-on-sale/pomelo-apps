use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};

use crate::model::ScheduleEvent;

/// Parse an iCalendar (.ics) string and extract all events that occur on `today`.
pub fn parse_today_events(ics_content: &str, today: NaiveDate) -> Vec<ScheduleEvent> {
    let unfolded = unfold_lines(ics_content);
    let mut events = Vec::new();

    let mut in_vevent = false;
    let mut current_uid = String::new();
    let mut current_summary = String::new();
    let mut current_start: Option<(DateTime<Local>, bool)> = None; // (datetime, is_all_day)
    let mut current_end: Option<(DateTime<Local>, bool)> = None;

    for line in unfolded.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "BEGIN:VEVENT" {
            in_vevent = true;
            current_uid.clear();
            current_summary.clear();
            current_start = None;
            current_end = None;
            continue;
        }

        if trimmed == "END:VEVENT" {
            if in_vevent {
                if let Some((start_dt, is_all_day)) = current_start {
                    let end_dt = match current_end {
                        Some((dt, _)) => dt,
                        None => {
                            if is_all_day {
                                start_dt + Duration::days(1)
                            } else {
                                start_dt + Duration::hours(1)
                            }
                        }
                    };

                    if occurs_on_date(start_dt, end_dt, is_all_day, today) {
                        let title = if current_summary.is_empty() {
                            "日程".to_string()
                        } else {
                            current_summary.clone()
                        };

                        let id = if current_uid.is_empty() {
                            format!("evt_{}", events.len())
                        } else {
                            current_uid.clone()
                        };

                        let color_index = compute_color_index(&id, &title);

                        events.push(ScheduleEvent {
                            id,
                            title,
                            start: start_dt,
                            end: end_dt,
                            is_all_day,
                            color_index,
                        });
                    }
                }
            }
            in_vevent = false;
            continue;
        }

        if !in_vevent {
            continue;
        }

        if let Some((prop_name, prop_val)) = parse_property_line(trimmed) {
            match prop_name.to_uppercase().as_str() {
                "UID" => current_uid = prop_val.to_string(),
                "SUMMARY" => current_summary = unescape_text(prop_val),
                name if name.starts_with("DTSTART") => {
                    if let Some(parsed) = parse_ics_datetime(prop_name, prop_val) {
                        current_start = Some(parsed);
                    }
                }
                name if name.starts_with("DTEND") => {
                    if let Some(parsed) = parse_ics_datetime(prop_name, prop_val) {
                        current_end = Some(parsed);
                    }
                }
                _ => {}
            }
        }
    }

    // Sort events by start time, with all-day events at the top
    events.sort_by(|a, b| {
        b.is_all_day.cmp(&a.is_all_day).then_with(|| a.start.cmp(&b.start))
    });

    events
}

/// Checks whether an event with start and end times covers or occurs on `target_date`.
fn occurs_on_date(
    start: DateTime<Local>,
    end: DateTime<Local>,
    is_all_day: bool,
    target_date: NaiveDate,
) -> bool {
    let start_date = start.date_naive();
    let end_date = end.date_naive();

    if is_all_day {
        // RFC 5545: DTEND for DATE is exclusive. If start is Oct 14 and end is Oct 15, it covers Oct 14 only.
        start_date <= target_date && end_date > target_date
    } else {
        // Timed event:
        // If event ends exactly at 00:00:00 on target_date, it finished right as today started
        if end_date == target_date && end.time() == chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap() {
            return false;
        }
        start_date <= target_date && end_date >= target_date
    }
}

/// Computes a deterministic 0..4 color index for visual variety.
fn compute_color_index(id: &str, title: &str) -> usize {
    let mut hash: u32 = 5381;
    for b in id.bytes().chain(title.bytes()) {
        hash = hash.wrapping_mul(33).wrapping_add(b as u32);
    }
    (hash % 4) as usize
}

/// Unfold lines that were wrapped per RFC 5545 section 3.1.
fn unfold_lines(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len());
    for line in raw.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            output.push_str(&line[1..]);
        } else {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(line);
        }
    }
    output
}

/// Parse a property line into (property_key_part, property_value).
fn parse_property_line(line: &str) -> Option<(&str, &str)> {
    let colon_pos = line.find(':')?;
    let key = &line[..colon_pos];
    let val = &line[colon_pos + 1..];
    Some((key, val))
}

/// Parse an RFC 5545 date or datetime value.
///
/// Handles:
/// - `YYYYMMDD` (Date only, e.g. 20261014)
/// - `YYYYMMDDTHHMMSS` (Local floating)
/// - `YYYYMMDDTHHMMSSZ` (UTC time, converted to Local)
fn parse_ics_datetime(key: &str, val: &str) -> Option<(DateTime<Local>, bool)> {
    let trimmed_val = val.trim();

    // Check if property key specifies VALUE=DATE (e.g. DTSTART;VALUE=DATE:20261014)
    if key.contains("VALUE=DATE") || trimmed_val.len() == 8 {
        if let Ok(naive_date) = NaiveDate::parse_from_str(trimmed_val, "%Y%m%d") {
            let naive_dt = naive_date.and_hms_opt(0, 0, 0)?;
            let local_dt = Local.from_local_datetime(&naive_dt).single()?;
            return Some((local_dt, true));
        }
    }

    // UTC Datetime ending with 'Z'
    if trimmed_val.ends_with('Z') {
        let without_z = &trimmed_val[..trimmed_val.len() - 1];
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(without_z, "%Y%m%dT%H%M%S") {
            let utc_dt = Utc.from_utc_datetime(&naive_dt);
            let local_dt = utc_dt.with_timezone(&Local);
            return Some((local_dt, false));
        }
    }

    // Local / Floating Datetime
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(trimmed_val, "%Y%m%dT%H%M%S") {
        if let Some(local_dt) = Local.from_local_datetime(&naive_dt).single() {
            return Some((local_dt, false));
        }
    }

    None
}

/// Unescape text per RFC 5545 section 3.3.11.
fn unescape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next) = chars.peek() {
                match next {
                    ',' => {
                        out.push(',');
                        chars.next();
                    }
                    ';' => {
                        out.push(';');
                        chars.next();
                    }
                    '\\' => {
                        out.push('\\');
                        chars.next();
                    }
                    'n' | 'N' => {
                        out.push(' ');
                        chars.next();
                    }
                    _ => out.push(ch),
                }
            } else {
                out.push(ch);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard_vevent_fixture() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let ics = r#"
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Apple Inc.//macOS 14.0//EN
BEGIN:VEVENT
UID:evt-today-1
SUMMARY:项目例会
DTSTART;TZID=Asia/Shanghai:20261014T093000
DTEND;TZID=Asia/Shanghai:20261014T103000
END:VEVENT
BEGIN:VEVENT
UID:evt-today-all-day
SUMMARY:全天发布日
DTSTART;VALUE=DATE:20261014
DTEND;VALUE=DATE:20261015
END:VEVENT
BEGIN:VEVENT
UID:evt-tomorrow
SUMMARY:明天日程
DTSTART;TZID=Asia/Shanghai:20261015T140000
DTEND;TZID=Asia/Shanghai:20261015T150000
END:VEVENT
END:VCALENDAR
"#;

        let events = parse_today_events(ics, today);
        assert_eq!(events.len(), 2, "Should only extract today's 2 events");

        // All day should be sorted first
        assert_eq!(events[0].title, "全天发布日");
        assert!(events[0].is_all_day);

        // Timed event second
        assert_eq!(events[1].title, "项目例会");
        assert!(!events[1].is_all_day);
    }

    #[test]
    fn parse_outlook_fixture() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        let ics = r#"BEGIN:VCALENDAR
METHOD:PUBLISH
PRODID:Microsoft Exchange Server 2010
VERSION:2.0
X-WR-CALNAME:日历
BEGIN:VTIMEZONE
TZID:China Standard Time
BEGIN:STANDARD
DTSTART:16010101T000000
TZOFFSETFROM:+0800
TZOFFSETTO:+0800
END:STANDARD
BEGIN:DAYLIGHT
DTSTART:16010101T000000
TZOFFSETFROM:+0800
TZOFFSETTO:+0800
END:DAYLIGHT
END:VTIMEZONE
BEGIN:VEVENT
DESCRIPTION:\n
UID:040000008200E00074C5B7101A82E0080000000029DBE6E8745BDD01000000000000000
 010000000E541DA2832B36E49B4FC7CB0AD93E36C
SUMMARY:今日会议
DTSTART;TZID=China Standard Time:20261014T090000
DTEND;TZID=China Standard Time:20261014T100000
CLASS:PUBLIC
PRIORITY:5
DTSTAMP:20261014T012812Z
TRANSP:OPAQUE
STATUS:CONFIRMED
SEQUENCE:0
LOCATION:
X-MICROSOFT-CDO-APPT-SEQUENCE:0
X-MICROSOFT-CDO-BUSYSTATUS:BUSY
X-MICROSOFT-CDO-INTENDEDSTATUS:BUSY
X-MICROSOFT-CDO-ALLDAYEVENT:FALSE
X-MICROSOFT-CDO-IMPORTANCE:1
X-MICROSOFT-CDO-INSTTYPE:0
X-MICROSOFT-DONOTFORWARDMEETING:FALSE
X-MICROSOFT-DISALLOW-COUNTER:FALSE
X-MICROSOFT-REQUESTEDATTENDANCEMODE:DEFAULT
X-MICROSOFT-ISRESPONSEREQUESTED:FALSE
END:VEVENT
END:VCALENDAR
"#;
        let events = parse_today_events(ics, today);
        println!("PARSED OUTLOOK EVENTS: {:?}", events);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "今日会议");
    }
}
