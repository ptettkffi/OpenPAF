use chrono::{TimeZone, Utc};
use chrono::DateTime as ChronoDateTime;
use chrono::Datelike;
use chrono::Duration;
use chrono_tz::Tz;
use std::error::Error;
use super::timefreq::TimeFreq;

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub struct DateTime {
    dt: ChronoDateTime<Utc>
}

impl DateTime {
    fn read_timezone(timezone: Option<&str>) -> Result<Tz, Box<Error>> {
        let tz_str = timezone.unwrap_or("UTC");
        let tz: Tz = tz_str.parse()?;
        Ok(tz)
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
        let parsed: TimeFreq = TimeFreq::from_timestamp(timestamp)?;

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
                self.dt = self.dt.with_month(self.dt.month() + (parsed.months - 12)).unwrap();
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
        let parsed: TimeFreq = TimeFreq::from_timestamp(timestamp)?;

        // Handle years
        if parsed.years > 0 {
            self.dt = self.dt.with_year(self.dt.year() - parsed.years as i32).unwrap();
        }

        // Handle months
        if parsed.months > 0 {
            if self.dt.month() - parsed.months <= 0 {
                // We cannot have more than 12 months at this point
                self.dt = self.dt.with_year(self.dt.year() - 1).unwrap();

                // Add or subtract the difference in months
                self.dt = self.dt.with_month(self.dt.month() + (12 - parsed.months)).unwrap();
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
}
