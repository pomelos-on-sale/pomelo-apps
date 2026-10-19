use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};

use crate::model::{ColorRule, ScheduleEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByDayItem {
    pub ordinal: Option<i8>,
    pub weekday: chrono::Weekday,
}

impl ByDayItem {
    pub fn matches(&self, date: NaiveDate) -> bool {
        if date.weekday() != self.weekday {
            return false;
        }
        match self.ordinal {
            None => true,
            Some(n) if n > 0 => {
                let mut count = 0;
                let mut d = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap();
                while d <= date {
                    if d.weekday() == self.weekday {
                        count += 1;
                    }
                    d += Duration::days(1);
                }
                count == (n as usize)
            }
            Some(n) if n < 0 => {
                let mut count = 0;
                let mut d = date;
                let next_month = if date.month() == 12 {
                    NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1).unwrap()
                };
                while d < next_month {
                    if d.weekday() == self.weekday {
                        count += 1;
                    }
                    d += Duration::days(1);
                }
                count == ((-n) as usize)
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceRule {
    pub freq: Freq,
    pub interval: usize,
    pub until: Option<NaiveDate>,
    pub count: Option<usize>,
    pub by_day: Vec<ByDayItem>,
    pub by_month_day: Vec<u32>,
    pub by_month: Vec<u32>,
}

impl RecurrenceRule {
    pub fn matches_date(&self, start_date: NaiveDate, target_date: NaiveDate) -> bool {
        if target_date < start_date {
            return false;
        }

        if let Some(until) = self.until {
            if target_date > until {
                return false;
            }
        }

        if !self.matches_pattern(start_date, target_date) {
            return false;
        }

        if let Some(max_count) = self.count {
            let mut count = 0;
            let mut d = start_date;
            while d <= target_date {
                if self.matches_pattern(start_date, d) {
                    count += 1;
                    if count > max_count {
                        return false;
                    }
                }
                d += Duration::days(1);
            }
            count <= max_count
        } else {
            true
        }
    }

    fn matches_pattern(&self, start_date: NaiveDate, target_date: NaiveDate) -> bool {
        if target_date < start_date {
            return false;
        }
        if let Some(until) = self.until {
            if target_date > until {
                return false;
            }
        }

        if !self.by_month.is_empty() && !self.by_month.contains(&target_date.month()) {
            return false;
        }

        match self.freq {
            Freq::Daily => {
                if !self.by_day.is_empty()
                    && !self.by_day.iter().any(|item| item.weekday == target_date.weekday())
                {
                    return false;
                }
                let days_diff = (target_date - start_date).num_days();
                days_diff >= 0 && days_diff % (self.interval as i64) == 0
            }
            Freq::Weekly => {
                if self.by_day.is_empty() {
                    if target_date.weekday() != start_date.weekday() {
                        return false;
                    }
                } else if !self.by_day.iter().any(|item| item.weekday == target_date.weekday()) {
                    return false;
                }

                let start_monday =
                    start_date - Duration::days(start_date.weekday().num_days_from_monday() as i64);
                let target_monday =
                    target_date - Duration::days(target_date.weekday().num_days_from_monday() as i64);
                let weeks_diff = (target_monday - start_monday).num_days() / 7;
                weeks_diff >= 0 && weeks_diff % (self.interval as i64) == 0
            }
            Freq::Monthly => {
                let total_months_diff = (target_date.year() - start_date.year()) * 12
                    + (target_date.month() as i32 - start_date.month() as i32);
                if total_months_diff < 0 || total_months_diff % (self.interval as i32) != 0 {
                    return false;
                }

                if !self.by_month_day.is_empty() {
                    self.by_month_day.contains(&target_date.day())
                } else if !self.by_day.is_empty() {
                    self.by_day.iter().any(|item| item.matches(target_date))
                } else {
                    target_date.day() == start_date.day()
                }
            }
            Freq::Yearly => {
                let years_diff = target_date.year() - start_date.year();
                if years_diff < 0 || years_diff % (self.interval as i32) != 0 {
                    return false;
                }

                if self.by_month.is_empty() && target_date.month() != start_date.month() {
                    return false;
                }

                if !self.by_day.is_empty() {
                    self.by_day.iter().any(|item| item.matches(target_date))
                } else if !self.by_month_day.is_empty() {
                    self.by_month_day.contains(&target_date.day())
                } else {
                    target_date.day() == start_date.day()
                }
            }
        }
    }
}

/// Parse an iCalendar (.ics) string and extract events that occur on `today` and `tomorrow`.
pub fn parse_schedule_events(
    ics_content: &str,
    today: NaiveDate,
) -> (Vec<ScheduleEvent>, Vec<ScheduleEvent>) {
    parse_schedule_events_with_rules(ics_content, today, &[])
}

/// Parse an iCalendar (.ics) string and extract events that occur on `today` and `tomorrow`,
/// applying custom keyword-to-color preset rules.
pub fn parse_schedule_events_with_rules(
    ics_content: &str,
    today: NaiveDate,
    rules: &[ColorRule],
) -> (Vec<ScheduleEvent>, Vec<ScheduleEvent>) {
    let tomorrow = today + Duration::days(1);
    let unfolded = unfold_lines(ics_content);
    let mut today_events = Vec::new();
    let mut tomorrow_events = Vec::new();

    let mut in_vevent = false;
    let mut current_uid = String::new();
    let mut current_summary = String::new();
    let mut current_start: Option<(DateTime<Local>, bool)> = None; // (datetime, is_all_day)
    let mut current_end: Option<(DateTime<Local>, bool)> = None;
    let mut current_rrule: Option<RecurrenceRule> = None;
    let mut current_exdates: Vec<NaiveDate> = Vec::new();

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
            current_rrule = None;
            current_exdates.clear();
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

                    let title = if current_summary.is_empty() {
                        "日程".to_string()
                    } else {
                        current_summary.clone()
                    };

                    let id = if current_uid.is_empty() {
                        format!("evt_{}", today_events.len() + tomorrow_events.len())
                    } else {
                        current_uid.clone()
                    };

                    let color_index = compute_color_index_with_rules(&title, rules);

                    for evt in collect_event_instances_for_date(
                        &id,
                        &title,
                        start_dt,
                        end_dt,
                        is_all_day,
                        current_rrule.as_ref(),
                        &current_exdates,
                        today,
                        color_index,
                    ) {
                        today_events.push(evt);
                    }

                    for evt in collect_event_instances_for_date(
                        &id,
                        &title,
                        start_dt,
                        end_dt,
                        is_all_day,
                        current_rrule.as_ref(),
                        &current_exdates,
                        tomorrow,
                        color_index,
                    ) {
                        tomorrow_events.push(evt);
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
                name if name.starts_with("RRULE") => {
                    if let Some(rule) = parse_rrule(prop_val) {
                        current_rrule = Some(rule);
                    }
                }
                name if name.starts_with("EXDATE") => {
                    for token in prop_val.split(',') {
                        let token = token.trim();
                        if token.is_empty() {
                            continue;
                        }
                        if let Some((dt, _)) = parse_ics_datetime(prop_name, token) {
                            current_exdates.push(dt.date_naive());
                        } else {
                            let date_str = if let Some(t_pos) = token.find('T') {
                                &token[..t_pos]
                            } else {
                                token
                            };
                            if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y%m%d") {
                                current_exdates.push(d);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Sort events by start time, with all-day events at the top
    today_events.sort_by(|a, b| {
        b.is_all_day.cmp(&a.is_all_day).then_with(|| a.start.cmp(&b.start))
    });
    tomorrow_events.sort_by(|a, b| {
        b.is_all_day.cmp(&a.is_all_day).then_with(|| a.start.cmp(&b.start))
    });

    (today_events, tomorrow_events)
}

/// Parse an iCalendar (.ics) string and extract all events that occur on `today`.
pub fn parse_today_events(ics_content: &str, today: NaiveDate) -> Vec<ScheduleEvent> {
    parse_schedule_events(ics_content, today).0
}

/// Parse an iCalendar (.ics) string with custom color preset rules and extract all events that occur on `today`.
pub fn parse_today_events_with_rules(
    ics_content: &str,
    today: NaiveDate,
    rules: &[ColorRule],
) -> Vec<ScheduleEvent> {
    parse_schedule_events_with_rules(ics_content, today, rules).0
}

/// Collects valid event instances for `target_date`, expanding recurring rules if present.
#[allow(clippy::too_many_arguments)]
fn collect_event_instances_for_date(
    base_id: &str,
    title: &str,
    start_dt: DateTime<Local>,
    end_dt: DateTime<Local>,
    is_all_day: bool,
    rrule: Option<&RecurrenceRule>,
    exdates: &[NaiveDate],
    target_date: NaiveDate,
    color_index: usize,
) -> Vec<ScheduleEvent> {
    let mut instances = Vec::new();

    if let Some(rule) = rrule {
        let duration = end_dt - start_dt;
        let span_days = (end_dt.date_naive() - start_dt.date_naive()).num_days().max(0);
        let start_time = start_dt.time();

        let check_start = target_date - Duration::days(span_days);
        let mut d = check_start;
        while d <= target_date {
            if !exdates.contains(&d) && rule.matches_date(start_dt.date_naive(), d) {
                let occ_naive_start = d.and_time(start_time);
                if let Some(occ_start) = Local.from_local_datetime(&occ_naive_start).earliest() {
                    let occ_end = occ_start + duration;
                    if occurs_on_date(occ_start, occ_end, is_all_day, target_date) {
                        instances.push(ScheduleEvent {
                            id: format!("{}_{}", base_id, d.format("%Y%m%d")),
                            title: title.to_string(),
                            start: occ_start,
                            end: occ_end,
                            is_all_day,
                            color_index,
                        });
                    }
                }
            }
            d += Duration::days(1);
        }
    } else {
        if occurs_on_date(start_dt, end_dt, is_all_day, target_date) {
            instances.push(ScheduleEvent {
                id: base_id.to_string(),
                title: title.to_string(),
                start: start_dt,
                end: end_dt,
                is_all_day,
                color_index,
            });
        }
    }

    instances
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

/// Computes a deterministic color index based on custom preset rules first,
/// falling back to a djb2 hash modulo 7 if no keyword matches.
pub fn compute_color_index_with_rules(title: &str, rules: &[ColorRule]) -> usize {
    let title_lower = title.to_lowercase();
    for rule in rules {
        let kw = rule.keyword.trim();
        if !kw.is_empty() && title_lower.contains(&kw.to_lowercase()) {
            return rule.color_index % 7;
        }
    }
    compute_color_index(title)
}

/// Computes a deterministic 0..7 color index based solely on title, so identical titles share the same color.
pub fn compute_color_index(title: &str) -> usize {
    let mut hash: u32 = 5381;
    for b in title.trim().bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u32);
    }
    (hash % 7) as usize
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

fn parse_by_day_item(s: &str) -> Option<ByDayItem> {
    let s = s.trim();
    if s.len() < 2 {
        return None;
    }
    let (prefix, day_str) = s.split_at(s.len() - 2);
    let weekday = match day_str.to_uppercase().as_str() {
        "SU" => chrono::Weekday::Sun,
        "MO" => chrono::Weekday::Mon,
        "TU" => chrono::Weekday::Tue,
        "WE" => chrono::Weekday::Wed,
        "TH" => chrono::Weekday::Thu,
        "FR" => chrono::Weekday::Fri,
        "SA" => chrono::Weekday::Sat,
        _ => return None,
    };
    let ordinal = if prefix.is_empty() {
        None
    } else {
        prefix.parse::<i8>().ok()
    };
    Some(ByDayItem { ordinal, weekday })
}

/// Parses an RFC 5545 RRULE string into `RecurrenceRule`.
pub fn parse_rrule(val: &str) -> Option<RecurrenceRule> {
    let mut freq = None;
    let mut interval = 1;
    let mut until = None;
    let mut count = None;
    let mut by_day = Vec::new();
    let mut by_month_day = Vec::new();
    let mut by_month = Vec::new();

    for part in val.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let mut kv = part.splitn(2, '=');
        let key = kv.next()?.trim().to_uppercase();
        let val = kv.next()?.trim();

        match key.as_str() {
            "FREQ" => {
                freq = match val.to_uppercase().as_str() {
                    "DAILY" => Some(Freq::Daily),
                    "WEEKLY" => Some(Freq::Weekly),
                    "MONTHLY" => Some(Freq::Monthly),
                    "YEARLY" => Some(Freq::Yearly),
                    _ => None,
                };
            }
            "INTERVAL" => {
                if let Ok(inv) = val.parse::<usize>() {
                    interval = inv.max(1);
                }
            }
            "UNTIL" => {
                let date_str = if let Some(t_pos) = val.find('T') {
                    &val[..t_pos]
                } else {
                    val
                };
                if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y%m%d") {
                    until = Some(d);
                }
            }
            "COUNT" => {
                if let Ok(cnt) = val.parse::<usize>() {
                    count = Some(cnt);
                }
            }
            "BYDAY" => {
                for item in val.split(',') {
                    if let Some(by_day_item) = parse_by_day_item(item) {
                        by_day.push(by_day_item);
                    }
                }
            }
            "BYMONTHDAY" => {
                for item in val.split(',') {
                    if let Ok(day) = item.trim().parse::<u32>() {
                        by_month_day.push(day);
                    }
                }
            }
            "BYMONTH" => {
                for item in val.split(',') {
                    if let Ok(m) = item.trim().parse::<u32>() {
                        by_month.push(m);
                    }
                }
            }
            _ => {}
        }
    }

    Some(RecurrenceRule {
        freq: freq?,
        interval,
        until,
        count,
        by_day,
        by_month_day,
        by_month,
    })
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
            let local_dt = Local.from_local_datetime(&naive_dt).earliest()?;
            return Some((local_dt, true));
        }
    }

    // UTC Datetime ending with 'Z'
    if let Some(without_z) = trimmed_val.strip_suffix('Z') {
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(without_z, "%Y%m%dT%H%M%S") {
            let utc_dt = Utc.from_utc_datetime(&naive_dt);
            let local_dt = utc_dt.with_timezone(&Local);
            return Some((local_dt, false));
        }
    }

    // Local / Floating Datetime
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(trimmed_val, "%Y%m%dT%H%M%S") {
        if let Some(local_dt) = Local.from_local_datetime(&naive_dt).earliest() {
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

        // Test extracting both today and tomorrow
        let (today_evts, tomorrow_evts) = parse_schedule_events(ics, today);
        assert_eq!(today_evts.len(), 2);
        assert_eq!(tomorrow_evts.len(), 1);
        assert_eq!(tomorrow_evts[0].title, "明天日程");
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

    #[test]
    fn parse_rrule_daily_and_weekly() {
        let rrule_daily = parse_rrule("FREQ=DAILY").unwrap();
        assert_eq!(rrule_daily.freq, Freq::Daily);
        assert_eq!(rrule_daily.interval, 1);

        let start = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
        assert!(rrule_daily.matches_date(start, NaiveDate::from_ymd_opt(2026, 10, 17).unwrap()));
        assert!(!rrule_daily.matches_date(start, NaiveDate::from_ymd_opt(2026, 10, 12).unwrap()));

        let rrule_weekdays = parse_rrule("FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR").unwrap();
        assert_eq!(rrule_weekdays.freq, Freq::Weekly);
        assert_eq!(rrule_weekdays.by_day.len(), 5);

        // 2026-10-17 is Saturday -> false
        assert!(!rrule_weekdays.matches_date(start, NaiveDate::from_ymd_opt(2026, 10, 17).unwrap()));
        // 2026-10-19 is Monday -> true
        assert!(rrule_weekdays.matches_date(start, NaiveDate::from_ymd_opt(2026, 10, 19).unwrap()));
    }

    #[test]
    fn parse_recurring_events_from_user_calendar() {
        let ics = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//caldav.icloud.com//CALDAVJ 2638B987//EN
BEGIN:VEVENT
UID:17B77668-1B39-4A3C-85D2-D6CC028487D3
SUMMARY:学习
DTSTART;TZID=Asia/Shanghai:20261013T095000
DTEND;TZID=Asia/Shanghai:20261013T104300
RRULE:FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR
END:VEVENT
BEGIN:VEVENT
UID:1C42AD4D-8EF6-4551-95EA-94071819136A
SUMMARY:午饭午休
DTSTART;TZID=Asia/Shanghai:20261013T110000
DTEND;TZID=Asia/Shanghai:20261013T130000
RRULE:FREQ=WEEKLY;BYDAY=SU,MO,TU,WE,TH,FR,SA
END:VEVENT
BEGIN:VEVENT
UID:C1C4CED2-B9A3-453C-9B95-4F2472D6DDFA
SUMMARY:晚饭晚休
DTSTART;TZID=Asia/Shanghai:20261013T165000
DTEND;TZID=Asia/Shanghai:20261013T182000
RRULE:FREQ=DAILY
END:VEVENT
BEGIN:VEVENT
UID:5D4BE8CA-6F86-4DDF-94B6-85BC0C764196
SUMMARY:背单词
DTSTART;TZID=Asia/Shanghai:20261013T213000
DTEND;TZID=Asia/Shanghai:20261013T220000
RRULE:FREQ=DAILY
END:VEVENT
BEGIN:VEVENT
UID:7DBDB689-2468-4939-980F-900FCD6BD420
SUMMARY:Clean teeth
DTSTART;TZID=America/Toronto:20220901T140000
DTEND;TZID=America/Toronto:20220901T150000
END:VEVENT
END:VCALENDAR"#;

        // Saturday Oct 17, 2026
        let saturday = NaiveDate::from_ymd_opt(2026, 10, 17).unwrap();
        let (today_evts, tomorrow_evts) = parse_schedule_events(ics, saturday);

        // Saturday: should have 3 events (午饭午休, 晚饭晚休, 背单词)
        assert_eq!(today_evts.len(), 3);
        assert_eq!(today_evts[0].title, "午饭午休");
        assert_eq!(today_evts[1].title, "晚饭晚休");
        assert_eq!(today_evts[2].title, "背单词");

        // Sunday (tomorrow): should also have the same 3 events
        assert_eq!(tomorrow_evts.len(), 3);
        assert_eq!(tomorrow_evts[0].title, "午饭午休");
        assert_eq!(tomorrow_evts[1].title, "晚饭晚休");
        assert_eq!(tomorrow_evts[2].title, "背单词");

        // Monday Oct 19, 2026: weekday should include 学习 as well
        let monday = NaiveDate::from_ymd_opt(2026, 10, 19).unwrap();
        let (mon_evts, _) = parse_schedule_events(ics, monday);
        assert_eq!(mon_evts.len(), 4);
        assert_eq!(mon_evts[0].title, "学习");
    }

    #[test]
    fn color_index_is_consistent_for_same_title() {
        assert_eq!(compute_color_index("站立例会"), compute_color_index("站立例会"));
        assert_eq!(compute_color_index(" 站立例会 "), compute_color_index("站立例会"));
    }

    #[test]
    fn color_rule_matching_and_fallback() {
        let rules = vec![
            ColorRule {
                keyword: "会议".to_string(),
                color_index: 0, // Blue
            },
            ColorRule {
                keyword: "Code Review".to_string(),
                color_index: 1, // Green
            },
        ];

        // Matches first rule keyword "会议"
        assert_eq!(compute_color_index_with_rules("下午部门会议", &rules), 0);
        // Matches second rule (case-insensitive)
        assert_eq!(compute_color_index_with_rules("Team code review", &rules), 1);
        // No match -> falls back to hash
        assert_eq!(
            compute_color_index_with_rules("健身锻炼", &rules),
            compute_color_index("健身锻炼")
        );
    }
}
