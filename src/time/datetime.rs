use chrono::{TimeZone, Utc, Datelike, Timelike, Duration};
use chrono::DateTime as ChronoDateTime;
use chrono_tz::Tz;
use std::error::Error;
use super::timefreq::{TimeFreq, Resolution};
use super::super::error::PafError;

/// Constant for the application's accepted time format.
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// A simple wrapper around chrono::DateTime, allowing for
/// creating methods without overriding anything by accident
pub struct DateTime {
    dt: ChronoDateTime<Utc>
}

impl DateTime {
    /// Parses a timezone string, and returns a wrapped chrono_tz::Tz object, if it is valid
    /// Returns a wrapped error, if it is not valid
    /// 
    /// # Arguments
    /// 
    /// * `timezone` - A string representation of a valid timezone (e.g. UTC, GMT, CET)
    fn read_timezone(timezone: Option<&str>) -> Result<Tz, Box<Error>> {
        let tz_str = timezone.unwrap_or("UTC");
        let tz: Tz = tz_str.parse()?;
        Ok(tz)
    }

    fn merge_error(result: Option<ChronoDateTime<Utc>>, msg: String) -> Result<ChronoDateTime<Utc>, Box<PafError>> {
        if result.is_none() {
            return Err(PafError::create_error(&msg));
        }
        return Ok(result.unwrap())
    }

    fn merge_timefreq(&mut self, timefreq: &TimeFreq) -> Result<(), Box<Error>> {
        // Local variables for error handling
        let mut result;
        let res = &timefreq.resolution;

        if res >= &Resolution::Second {
            result = self.dt.with_second(timefreq.seconds);
            self.dt = DateTime::merge_error(result, format!("Invalid number of seconds {}.", timefreq.seconds))?;
        }

        if res >= &Resolution::Minute {
            result = self.dt.with_minute(timefreq.minutes);
            self.dt = DateTime::merge_error(result, format!("Invalid number of minutes {}.", timefreq.minutes))?;
        }

        if res >= &Resolution::Hour {
            result = self.dt.with_hour(timefreq.hours);
            self.dt = DateTime::merge_error(result, format!("Invalid number of hours {}.", timefreq.hours))?;
        }

        if res >= &Resolution::Day {
            result = self.dt.with_day(timefreq.days);
            self.dt = DateTime::merge_error(result, format!("Invalid number of days {}.", timefreq.days))?;
        }

        if res >= &Resolution::Month {
            result = self.dt.with_month(timefreq.months);
            self.dt = DateTime::merge_error(result, format!("Invalid number of months {}.", timefreq.months))?;
        }

        if res >= &Resolution::Year {
            result = self.dt.with_year(timefreq.years as i32);
            self.dt = DateTime::merge_error(result, format!("Invalid number of years {}.", timefreq.years))?;
        }

        Ok(())
    }

    pub fn from_timestamp(ts: &str, timezone: Option<&str>) -> Result<DateTime, Box<Error>> {
        let tz: Tz = DateTime::read_timezone(timezone)?;
        let dt = tz.datetime_from_str(ts, TIMESTAMP_FORMAT)?.with_timezone(&Utc);
        Ok(DateTime {dt: dt})
    }

    pub fn from_epoch(epoch: i64) -> DateTime {
        DateTime {dt: Utc.timestamp(epoch, 0)}
    }

    pub fn now() -> DateTime {
        DateTime {dt: Utc::now()}
    }

    pub fn to_timestamp(&self, timezone: Option<&str>) -> Result<String, Box<Error>> {
        let tz: Tz = DateTime::read_timezone(timezone)?;
        let stamp = self.dt.with_timezone(&tz).format(TIMESTAMP_FORMAT).to_string();
        Ok(stamp)
    }

    pub fn to_epoch(&self) -> i64 {
        self.dt.timestamp()
    }

    pub fn add(&mut self, timestamp: &str) -> Result<(), Box<Error>> {
        let parsed: TimeFreq = TimeFreq::from_timestamp(timestamp, true)?;

        // Handle years
        if parsed.years > 0 {
            self.dt = self.dt.with_year(self.dt.year() + parsed.years as i32).unwrap();
        }

        // Handle months
        if parsed.months > 0 {
            if parsed.months + self.dt.month() >= 12 {
                // We cannot have more than 12 months at this point
                self.dt = self.dt.with_year(self.dt.year() + 1).unwrap();

                // Add or subtract the difference in months
                self.dt = self.dt.with_month((self.dt.month() as i32 + (parsed.months as i32 - 12)) as u32).unwrap();
            } else {
                self.dt = self.dt.with_month(self.dt.month() + parsed.months).unwrap();
            }
        }

        // Add the rest of it as a single duration
        let dur = Duration::seconds(parsed.calc_duration());
        self.dt = self.dt + dur;
        Ok(())
    }

    pub fn subtract(&mut self, timestamp: &str) -> Result<(), Box<Error>> {
        let parsed: TimeFreq = TimeFreq::from_timestamp(timestamp, true)?;

        // Handle years
        if parsed.years > 0 {
            self.dt = self.dt.with_year(self.dt.year() - parsed.years as i32).unwrap();
        }

        // Handle months
        if parsed.months > 0 {
            if self.dt.month() as i32 - parsed.months as i32 <= 0 {
                // We cannot have more than 12 months at this point
                self.dt = self.dt.with_year(self.dt.year() - 1).unwrap();

                // Add or subtract the difference in months
                self.dt = self.dt.with_month((self.dt.month() as i32 + (12 - parsed.months as i32)) as u32).unwrap();
            } else {
                self.dt = self.dt.with_month(self.dt.month() - parsed.months).unwrap();
            }
        }

        // Subtract the rest of it as a single duration
        let dur = Duration::seconds(parsed.calc_duration());
        self.dt = self.dt - dur;
        Ok(())
    }

    pub fn is_passed(&self) -> bool {
        Utc::now() > self.dt
    }

    pub fn next_occurrence(timestamp: &str) -> Result<DateTime, Box<Error>> {
        let parsed: TimeFreq = TimeFreq::from_timestamp(timestamp, false)?;

        // Merge current time with available relative time components
        // e.g. if it's 2019-01-01 12:00:00 and the relative time is
        // 23:59:04, the result will be 2019-01-01 23:59:04
        let mut dt = DateTime::now();
        dt.merge_timefreq(&parsed)?;

        // If the previously constructed date and time is passed, add
        // one cycle according to its resolution
        // e.g. if the relative time is 23:59:04, add a day
        if dt.is_passed() {
            match parsed.resolution {
                Resolution::Year => return Err(PafError::create_error("Too specific timestamp, there is no next occurrence.")),
                Resolution::Month => dt.add("1-0-0 0:0:0").unwrap(),
                Resolution::Day => {
                    // We handle month additions differently, as days after 28 are not consistent
                    // in every month
                    let mut num_months = 1;
                    let mut res = dt.add(&format!("0-{}-0 0:0:0", num_months));
                    while res.is_err() {
                        num_months += 1;
                        res = dt.add(&format!("0-{}-0 0:0:0", num_months));
                    }

                },
                Resolution::Hour => dt.add("0-0-1 0:0:0").unwrap(),
                Resolution::Minute => dt.add("1:0:0").unwrap(),
                Resolution::Second => dt.add("0:1:0").unwrap(),
                Resolution::None => {}
            }
        }

        Ok(dt)
    }

}

#[cfg(test)]
mod tests {
    mod epoch {
        use super::super::*;

        #[test]
        fn reads_from_epoch() {
            let timeobj = DateTime::from_epoch(1_500_000_000);
            assert_eq!(timeobj.to_epoch(), 1_500_000_000);
        }

        #[test]
        fn writes_to_epoch() {
            let timeobj = DateTime::from_timestamp("2017-07-14 02:40:00", None).unwrap();
            assert_eq!(timeobj.to_epoch(), 1_500_000_000);
        }
    }
    
    mod from_timestamp {
        use super::super::*;

        #[test]
        fn reads_from_timestamp() {
            let timeobj = DateTime::from_timestamp("2017-07-14 02:40:00", None).unwrap();
            assert_eq!(timeobj.to_epoch(), 1_500_000_000);
        }

        #[test]
        fn invalid_tz_throws_error() {
            let timeobj = DateTime::from_timestamp("2017-07-14 02:40:00", Some("Invalid"));
            assert!(timeobj.is_err());
        }
    }
    
    mod now {
        use super::super::*;

        #[test]
        fn creates_from_current_time() {
            let timeobj = DateTime::now();
            assert!(timeobj.to_epoch() == Utc::now().timestamp())
        }
    }

    mod to_timestamp {
        use super::super::*;

        #[test]
        fn converts_to_timestamp() {
            let timeobj = DateTime::from_epoch(1_500_000_000);
            let timestamp = timeobj.to_timestamp(None).unwrap();
            assert_eq!(timestamp, "2017-07-14 02:40:00");
        }

        #[test]
        fn handles_timezones_in_timestamp() {
            let timeobj = DateTime::from_timestamp("2017-07-14 02:40:00", Some("CET")).unwrap();
            let timestamp = timeobj.to_timestamp(None).unwrap();
            assert_eq!(timestamp, "2017-07-14 00:40:00");
        }

        #[test]
        fn handles_daylight_savings() {
            let timeobj = DateTime::from_timestamp("2017-03-14 02:40:00", Some("CET")).unwrap();
            let timestamp = timeobj.to_timestamp(None).unwrap();
            assert_eq!(timestamp, "2017-03-14 01:40:00");
        }

        #[test]
        fn converts_timestamp_to_timezone() {
            let timeobj = DateTime::from_epoch(1_500_000_000);
            let timestamp = timeobj.to_timestamp(Some("CET")).unwrap();
            assert_eq!(timestamp, "2017-07-14 04:40:00");
        }

        #[test]
        fn invalid_tz_throws_error() {
            let timeobj = DateTime::from_epoch(1_500_000_000);
            let timestamp = timeobj.to_timestamp(Some("Invalid"));
            assert!(timestamp.is_err());
        }
    }

    mod read_timezone {
        use super::super::*;

        #[test]
        fn reads_timezone() {
            let tz = DateTime::read_timezone(Some("CET")).unwrap();
            let dt = tz.timestamp(1_500_000_000, 0);
            assert_eq!(dt.format("%z").to_string(), "+0200");
        }

        #[test]
        fn defaults_to_utc() {
            let tz = DateTime::read_timezone(None).unwrap();
            let dt = tz.timestamp(1_500_000_000, 0);
            assert_eq!(dt.format("%z").to_string(), "+0000");
        }

        #[test]
        fn invalid_tz_throws_error() {
            let tz = DateTime::read_timezone(Some("Invalid"));
            assert!(tz.is_err());
        }
    }

    mod add {
        use super::super::*;

        #[test]
        fn adds_duration_to_date() {
            // 2017-07-14 02:40:00
            let mut timeobj = DateTime::from_epoch(1_500_000_000);
            timeobj.add("1-2-3 4:5:6").unwrap();
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-09-17 06:45:06");
        }

        #[test]
        fn wraps_around_year() {
            let mut timeobj = DateTime::from_epoch(1_500_000_000);
            timeobj.add("15-0").unwrap();
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-10-14 02:40:00");

            timeobj.add("30-0").unwrap();
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2021-04-14 02:40:00");

            timeobj.add("9-0").unwrap();
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2022-01-14 02:40:00");

            timeobj.add("20-0").unwrap();
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2023-09-14 02:40:00");
        }

        #[test]
        fn throws_error_on_invalid_ts() {
            let mut timeobj = DateTime::from_epoch(1_500_000_000);
            let res = timeobj.add("15?-0");
            assert!(res.is_err());
        }
    }

    mod subtract {
        use super::super::*;

        #[test]
        fn subs_duration_from_date() {
            // 2017-07-14 02:40:00
            let mut timeobj = DateTime::from_epoch(1_500_000_000);
            timeobj.subtract("1-2-3 4:5:6").unwrap();
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2016-05-10 22:34:54");
        }

        #[test]
        fn wraps_around_year() {
            let mut timeobj = DateTime::from_epoch(1_500_000_000);
            timeobj.subtract("15-0").unwrap();
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2016-04-14 02:40:00");

            timeobj.subtract("30-0").unwrap();
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2013-10-14 02:40:00");

            timeobj.subtract("11-0").unwrap();
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2012-11-14 02:40:00");

            timeobj.subtract("20-0").unwrap();
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2011-03-14 02:40:00");
        }

        #[test]
        fn throws_error_on_invalid_ts() {
            let mut timeobj = DateTime::from_epoch(1_500_000_000);
            let res = timeobj.subtract("15?-0");
            assert!(res.is_err());
        }
    }

    mod is_passed {
        use super::super::*;

        #[test]
        fn returns_false_if_not_passed() {
            let timeobj = DateTime::from_timestamp("2222-02-02 22:22:22", None).unwrap();
            assert!(!timeobj.is_passed());
        }

        #[test]
        fn returns_true_if_passed() {
            let timeobj = DateTime::from_timestamp("1972-02-02 22:22:22", None).unwrap();
            assert!(timeobj.is_passed());
        }
    }

    mod merge_error {
        use super::super::*;

        #[test]
        fn returns_result_if_not_null() {
            let chrono_res = Utc::now().with_minute(40);
            let merge_res = DateTime::merge_error(chrono_res, String::from("Some error message."));
            assert!(merge_res.is_ok())
        }

        #[test]
        fn returns_error_if_null() {
            let chrono_res = Utc::now().with_minute(70);
            let merge_res = DateTime::merge_error(chrono_res, String::from("Some error message."));
            assert!(merge_res.is_err());

            let err = merge_res.err().unwrap();
            assert_eq!(*err.message, String::from("Some error message."));
        }
    }

    mod merge_timefreq {
        use super::super::*;

        #[test]
        fn merges_seconds() {
            let old_now = Utc::now();
            let mut dt = DateTime::now();
            let mut tf = TimeFreq::from_timestamp("10", true).unwrap();
            dt.merge_timefreq(&mut tf).unwrap();

            assert_eq!(dt.dt.second(), 10);
            assert_eq!(dt.dt.minute(), old_now.minute());
            assert_eq!(dt.dt.hour(), old_now.hour());
            assert_eq!(dt.dt.day(), old_now.day());
            assert_eq!(dt.dt.month(), old_now.month());
            assert_eq!(dt.dt.year(), old_now.year());
        }

        #[test]
        fn merges_minutes() {
            let old_now = Utc::now();
            let mut dt = DateTime::now();
            let mut tf = TimeFreq::from_timestamp("20:0", true).unwrap();
            dt.merge_timefreq(&mut tf).unwrap();

            assert_eq!(dt.dt.second(), 0);
            assert_eq!(dt.dt.minute(), 20);
            assert_eq!(dt.dt.hour(), old_now.hour());
            assert_eq!(dt.dt.day(), old_now.day());
            assert_eq!(dt.dt.month(), old_now.month());
            assert_eq!(dt.dt.year(), old_now.year());
        }

        #[test]
        fn merges_hours() {
            let old_now = Utc::now();
            let mut dt = DateTime::now();
            let mut tf = TimeFreq::from_timestamp("12:0:0", true).unwrap();
            dt.merge_timefreq(&mut tf).unwrap();

            assert_eq!(dt.dt.second(), 0);
            assert_eq!(dt.dt.minute(), 0);
            assert_eq!(dt.dt.hour(), 12);
            assert_eq!(dt.dt.day(), old_now.day());
            assert_eq!(dt.dt.month(), old_now.month());
            assert_eq!(dt.dt.year(), old_now.year());
        }

        #[test]
        fn merges_days() {
            let old_now = Utc::now();
            let mut dt = DateTime::now();
            let mut tf = TimeFreq::from_timestamp("28 0:0:0", true).unwrap();
            dt.merge_timefreq(&mut tf).unwrap();

            assert_eq!(dt.dt.second(), 0);
            assert_eq!(dt.dt.minute(), 0);
            assert_eq!(dt.dt.hour(), 0);
            assert_eq!(dt.dt.day(), 28);
            assert_eq!(dt.dt.month(), old_now.month());
            assert_eq!(dt.dt.year(), old_now.year());
        }

        #[test]
        fn merges_months() {
            let old_now = Utc::now();
            let mut dt = DateTime::now();
            let mut tf = TimeFreq::from_timestamp("9-1 0:0:0", true).unwrap();
            dt.merge_timefreq(&mut tf).unwrap();

            assert_eq!(dt.dt.second(), 0);
            assert_eq!(dt.dt.minute(), 0);
            assert_eq!(dt.dt.hour(), 0);
            assert_eq!(dt.dt.day(), 1);
            assert_eq!(dt.dt.month(), 9);
            assert_eq!(dt.dt.year(), old_now.year());
        }

        #[test]
        fn merges_years() {
            let mut dt = DateTime::now();
            let mut tf = TimeFreq::from_timestamp("1001-1-1 0:0:0", true).unwrap();
            dt.merge_timefreq(&mut tf).unwrap();

            assert_eq!(dt.dt.second(), 0);
            assert_eq!(dt.dt.minute(), 0);
            assert_eq!(dt.dt.hour(), 0);
            assert_eq!(dt.dt.day(), 1);
            assert_eq!(dt.dt.month(), 1);
            assert_eq!(dt.dt.year(), 1001);
        }

        #[test]
        fn returns_error_if_cannot_be_merged() {
            let mut dt = DateTime::now();
            let mut tf = TimeFreq::from_timestamp("25:70:90", true).unwrap();
            let mut res = dt.merge_timefreq(&mut tf);
            assert!(res.is_err());

            //Day cannot be 0, only if it is not part of the relative date
            tf = TimeFreq::from_timestamp("0-1-0 0:0:0", true).unwrap();
            res = dt.merge_timefreq(&mut tf);
            assert!(res.is_err());

            //Month cannot be 0, only if it is not part of the relative date
            tf = TimeFreq::from_timestamp("1001-0-1 0:0:0", true).unwrap();
            res = dt.merge_timefreq(&mut tf);
            assert!(res.is_err());
        }
    }

    mod next_occurrence {
        use super::super::*;

        #[test]
        fn works_in_common_cases() {
            // Will fail at every exact hour, but currently IDC
            let mut now = Utc::now();
            let min = now.minute() - 1;
            let mut dt = DateTime::next_occurrence(&format!("{}:00", min)).unwrap();
            let mut expected = format!("{}-{:0width$}-{:0width$} {:0width$}:{:0width$}:{}",
                now.year(), now.month(), now.day(), now.hour() + 1, min, "00", width = 2);

            assert_eq!(dt.to_timestamp(None).unwrap(), expected);

            // TODO: test if it fails between midnight and 1am
            now = Utc::now();
            let hr;
            let mut exp_dt;
            if now.hour() == 0 {
                exp_dt = DateTime::now();
                hr = 1;
            } else {
                exp_dt = DateTime::now();
                exp_dt.add("1 00:00:00").unwrap();
                hr = now.hour() - 1;
            }
            dt = DateTime::next_occurrence(&format!("{}:00:00", hr)).unwrap();
            expected = format!("{}-{:0width$}-{:0width$} {:0width$}:{:0width$}:{}",
                exp_dt.dt.year(), exp_dt.dt.month(), exp_dt.dt.day(), hr, "00", "00", width = 2);
            assert_eq!(dt.to_timestamp(None).unwrap(), expected);
        }
    }
}
