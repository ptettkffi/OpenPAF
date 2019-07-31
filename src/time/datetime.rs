use chrono::{TimeZone, Utc, Datelike, Timelike, Duration, NaiveDate};
use chrono::DateTime as ChronoDateTime;
use chrono_tz::Tz;
use std::error::Error;
use super::timefreq::{TimeFreq, Resolution};
use super::super::error::PafError;

/// Constant for the application's accepted time format.
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// A simple wrapper around `chrono::DateTime`, allowing for
/// creating methods without overriding anything by accident
pub struct DateTime {
    dt: ChronoDateTime<Utc>
}

impl DateTime {
    /// Parses a timezone string, and returns a `chrono_tz::Tz` object, if it is valid.
    /// Returns an error, if it is not valid.
    /// 
    /// ## Arguments
    /// 
    /// * `timezone` - A string representation of a valid timezone (e.g. `"UTC"`, `"GMT"`, `"CET"`)
    /// 
    /// ## Examples
    /// ```
    /// let tz: Tz = _read_timezone(Some("CET")).unwrap(); // CET or CEST
    /// 
    /// let tz: Tz = _read_timezone(None).unwrap(); // UTC, same as providing Some("UTC")
    /// ```
    fn _read_timezone(timezone: Option<&str>) -> Result<Tz, Box<Error>> {
        let raw_str = timezone.unwrap_or("UTC");

        // For GMT+X and GMT-X timezones, preprend it with Etc/, like in the IANA DB
        // TODO: Consider inverting the sign
        let tz_str = if (raw_str.contains("+") || raw_str.contains("-")) && !raw_str.starts_with("Etc/")
            {String::from("Etc/") + raw_str} else {raw_str.to_string()};

        let tz: Tz = tz_str.parse()?;
        Ok(tz)
    }

    /// Utility function for checking a `chrono::DateTime` return value, and throwing an error, if there is none.
    /// 
    /// ## Arguments
    /// 
    /// * `result` - Result of a `chrono::DateTime` operation
    /// * `msg` - An error message, which should be provided, if the operation failed
    fn _merge_error(result: Option<ChronoDateTime<Utc>>, msg: String) -> Result<ChronoDateTime<Utc>, Box<PafError>> {
        // TODO: Move unwrapping and none-checking to caller methods, by using unwrap_or
        if result.is_none() {
            return Err(PafError::create_error(&msg));
        }
        Ok(result.unwrap())
    }

    /// Utility method for merging a partial datetime with a `DateTime` object.
    /// Used by `DateTime::next_occurrence()` for creating time references.
    /// 
    /// ## Arguments
    /// 
    /// * `timefreq` - A parsed partial datetime string to be merged
    /// 
    /// ## Examples
    /// ```
    /// let dt: DateTime = DateTime::now(); // Use current time as a starting point
    /// let tf: TimeFreq = TimeFreq::from_timestamp("02-11 10:30:00", false) // A parsed partial time string
    /// dt._merge_timefreq(tf).unwrap() // %Y-02-11 10:30:00, where %Y is current year
    /// ```
    fn _merge_timefreq(&mut self, timefreq: &TimeFreq) -> Result<(), Box<Error>> {
        // Local variables for error handling
        let mut result;
        let res = &timefreq.resolution;

        // NOTE: Order matters until hours
        if res >= &Resolution::Year {
            result = self.dt.with_year(timefreq.years as i32);
            self.dt = DateTime::_merge_error(result, format!("Invalid number of years {} in {}{}.", timefreq.years,
                timefreq.years, self.dt.format("-%m-%d %H:%M:%S")))?;
        }

        // We do not care about month-day pairing here, as month inputs also
        // have day inputs, and if the user provides 02-31, it is a user error
        if res >= &Resolution::Month {
            // Set days to 1, as it will be always valid, and if we have a month, we must also have a day for the next step
            result = self.dt.with_day(1).unwrap().with_month(timefreq.months);
            self.dt = DateTime::_merge_error(result, format!("Invalid number of months {} in {}{:0w$}{}.", timefreq.months,
                self.dt.format("%Y-"), timefreq.months, self.dt.format("-%d %H:%M:%S"), w = 2))?;
        }

        if res >= &Resolution::Day {
            // Handle days with care, since not every month have 31 days
            result = self.dt.with_day(timefreq.days);
            if result.is_none() && timefreq.days > self._get_last_day() && timefreq.days <= 31 {
                // Either we are in February or in a 30 days month with day 31 in the pattern
                let diff = timefreq.days - self._get_last_day();
                self._add_months(1);
                self.dt = self.dt.with_day(diff).unwrap();
            } else {
                self.dt = DateTime::_merge_error(result, format!("Invalid number of days {} in {}{:0w$}{}.", timefreq.days,
                self.dt.format("%Y-%m-"), timefreq.days, self.dt.format(" %H:%M:%S"), w = 2))?;
            }
        }

        if res >= &Resolution::Hour {
            result = self.dt.with_hour(timefreq.hours);
            self.dt = DateTime::_merge_error(result, format!("Invalid number of hours {} in {}{:0w$}{}.", timefreq.hours,
                self.dt.format("%Y-%m-%d "), timefreq.hours, self.dt.format(":%M:%S"), w = 2))?;
        }

        if res >= &Resolution::Minute {
            result = self.dt.with_minute(timefreq.minutes);
            self.dt = DateTime::_merge_error(result, format!("Invalid number of minutes {} in {}{:0w$}{}.", timefreq.minutes,
                self.dt.format("%Y-%m-%d %H:"), timefreq.minutes, self.dt.format(":%S"), w = 2))?;
        }

        if res >= &Resolution::Second {
            result = self.dt.with_second(timefreq.seconds);
            self.dt = DateTime::_merge_error(result, format!("Invalid number of seconds {} in {}{:0w$}.", timefreq.seconds,
                self.dt.format("%Y-%m-%d %H:%M:"), timefreq.seconds, w = 2))?;
        }

        Ok(())
    }

    /// Utility method for acquiring the last day in the current month (i.e. current month's size).
    /// 
    /// ## Examples
    /// ```
    /// let dt = DateTime::from_timestamp("2019-01-01 10:00:00", None).unwrap();
    /// dt._get_last_day() // 31
    /// ```
    fn _get_last_day(&self) -> u32 {
        NaiveDate::from_ymd_opt(self.dt.year(), self.dt.month() + 1, 1).unwrap_or(
            NaiveDate::from_ymd(self.dt.year() + 1, 1, 1)).pred().day() as u32
    }

    /// Utility method for adding months, since months can be wrapped least trivially in a date.
    /// (e.g. what is January 31 + 1 month?)
    /// 
    /// The program uses GNU date conventions; if a month transition cannot be done by incrementing the
    /// month, it moves forward by the numer of days in the mischevious month.
    /// 
    /// Cannot wrap around years, that is what `DateTime::add` is for.
    /// 
    /// ## Arguments
    /// * `months`: Number of months to add
    fn _add_months(&mut self, months: i32) {
        // Try to add every month at once
        let res = self.dt.with_month((self.dt.month() as i32 + months) as u32);

        if let Some(res_dt) = res {
            self.dt = res_dt;
        } else {
            // If it fails, iterate through the months and resolve the error in place
            for _ in 0..months {
                self.dt = self.dt.with_month(self.dt.month() + 1).unwrap_or(
                    self.dt + Duration::days(
                        self._get_last_day() as i64
                    )
                );
            }
        }
    }

    /// Utility method for subtracting months, since months can be wrapped least trivially in a date.
    /// (e.g. what is March 31 - 1 month?)
    /// 
    /// The program uses GNU date conventions; if a month transition cannot be done by decrementing the
    /// month, it moves back by the numer of days in the mischevious month.
    /// 
    /// Cannot wrap around years, that is what `DateTime::subtract` is for.
    /// 
    /// ## Arguments
    /// * `months`: Number of months to subtract
    fn _sub_months(&mut self, months: i32) {
        // Try to subtract every month at once
        let res = self.dt.with_month((self.dt.month() as i32 - months) as u32);

        if let Some(res_dt) = res {
            self.dt = res_dt;
        } else {
            // If it fails, iterate through the months and resolve the error in place
            for _ in 0..months {
                self.dt = self.dt.with_month(self.dt.month() - 1).unwrap_or(
                    self.dt - Duration::days(
                        self._get_last_day() as i64
                    )
                );
            }
        }
    }

    /// Utility method for calculating the next occurrence of a time pattern relative to
    /// a `DateTime` object. For more information, see `DateTime::next_occurrence`.
    fn _next_occurrence(timestamp: &str, ref_date: &DateTime) -> Result<DateTime, Box<Error>> {
        let parsed: TimeFreq = TimeFreq::from_timestamp(timestamp, false)?;

        // Merge current time with available relative time components
        // e.g. if it's 2019-01-01 12:00:00 and the relative time is
        // 23:59:04, the result will be 2019-01-01 23:59:04
        let mut merged = ref_date.clone();
        merged._merge_timefreq(&parsed)?;

        // If the previously constructed date and time is passed, add
        // one cycle according to its resolution
        // e.g. if the relative time is 23:59:04, add a day
        if merged.is_passed(Some(&ref_date)) {
            match parsed.resolution {
                Resolution::Year => return Err(PafError::create_error("Too specific timestamp, there is no next occurrence.")),
                Resolution::Month => merged.add("1-0-0 0:0:0").unwrap(),
                Resolution::Day => {
                    // We handle month additions differently, as days after 28 are not consistent
                    // in every month
                    let mut num_months = 1;
                    while merged.add(&format!("0-{}-0 0:0:0", num_months)).is_err() {
                        num_months += 1;
                    }

                },
                Resolution::Hour => merged.add("0-0-1 0:0:0").unwrap(),
                Resolution::Minute => merged.add("1:0:0").unwrap(),
                Resolution::Second => merged.add("0:1:0").unwrap(),
                Resolution::None => {}
            }
        }

        Ok(merged)
    }

    /// Clones the `DateTime` object.
    /// 
    /// ## Examples
    /// ```
    /// let dt = DateTime::now();
    /// let cloned = dt.clone() // We can now do whatever without modifying dt
    /// ```
    pub fn clone(&self) -> Self {
        Self { dt: self.dt.clone() }
    }

    /// Tries to create a new `DateTime` object from a string. On failure,
    /// it raises an error. If a timezone is provided, the string is
    /// treated as local, and converted to UTC.
    /// 
    /// Time string must be formatted the following way: %Y-%m-%d %H:%M:%S
    /// 
    /// For valid timezone strings, see the [IANA database](https://www.iana.org/time-zones)
    /// or a [browsable extract](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones).
    /// 
    /// ## Arguments
    /// * `ts` - A datetime string
    /// * `timezone` An optional timezone string
    /// 
    /// ## Examples
    /// ```
    /// let dt: DateTime = DateTime::from_timestamp("2019-01-01 12:00:00", None).unwrap();
    /// assert_eq!(dt.to_timestamp(None), "2019-01-01 12:00:00");
    /// 
    /// let dt: DateTime = DateTime::from_timestamp("2019-01-01 12:00:00", Some("CET")).unwrap();
    /// assert_eq!(dt.to_timestamp(None), "2019-01-01 11:00:00");
    /// ```
    pub fn from_timestamp(ts: &str, timezone: Option<&str>) -> Result<DateTime, Box<Error>> {
        let tz: Tz = DateTime::_read_timezone(timezone)?;
        let dt = tz.datetime_from_str(ts, TIMESTAMP_FORMAT)?.with_timezone(&Utc);
        Ok(DateTime {dt: dt})
    }

    /// Creates a new `DateTime` object from an integer. The integer is
    /// an epoch time, which is the number of seconds since January 1, 1970 UTC.
    pub fn from_epoch(epoch: i64) -> DateTime {
        DateTime {dt: Utc.timestamp(epoch, 0)}
    }

    /// Creates a `DateTime` object from the current time in UTC.
    pub fn now() -> DateTime {
        DateTime {dt: Utc::now()}
    }

    /// Serializes the `DateTime` object to a string. On failure,
    /// it raises an error. If a timezone is provided, the string represents
    /// time in the provided timezone.
    /// 
    /// Time string is formatted the following way: %Y-%m-%d %H:%M:%S
    /// 
    /// For valid timezone strings, see the [IANA database](https://www.iana.org/time-zones)
    /// or a [browsable extract](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones).
    /// 
    /// ## Arguments
    /// * `timezone` An optional timezone string
    /// 
    /// ## Examples
    /// ```
    /// let dt: DateTime = DateTime::from_timestamp("2019-01-01 12:00:00", None).unwrap();
    /// assert_eq!(dt.to_timestamp(None), "2019-01-01 12:00:00");
    /// 
    /// let dt: DateTime = DateTime::from_timestamp("2019-01-01 12:00:00", Some("CET")).unwrap();
    /// assert_eq!(dt.to_timestamp(Some("CET")), "2019-01-01 12:00:00");
    /// ```
    pub fn to_timestamp(&self, timezone: Option<&str>) -> Result<String, Box<Error>> {
        let tz: Tz = DateTime::_read_timezone(timezone)?;
        let stamp = self.dt.with_timezone(&tz).format(TIMESTAMP_FORMAT).to_string();
        Ok(stamp)
    }

    /// Calculates and returns the epoch time (UNIX timestamp) from the current
    /// `DateTime` object.
    pub fn to_epoch(&self) -> i64 {
        self.dt.timestamp()
    }

    /// Adds a partial time to the `DateTime` object. Partial times must be
    /// provided as strings in the general format %Y-%m-%d %H:%M:%S. The method
    /// is void, but on failure, it raises an error.
    /// 
    /// For the exact rules of partial time strings, see the `TimeFreq` documentation.
    /// 
    /// Wraps around years. For non-trivial behavior with months, see
    /// the documentation of `DateTime::_add_months`.
    /// 
    /// ## Arguments
    /// * `timestamp` A partial time string
    /// 
    /// ## Examples
    /// ```
    /// let dt: DateTime = DateTime::from_timestamp("2019-01-01 12:00:00", None).unwrap();
    /// dt.add("3:15:30").unwrap();
    /// assert_eq!(dt.to_timestamp(None), "2019-01-01 15:15:30");
    /// 
    /// let dt: DateTime = DateTime::from_timestamp("2019-08-01 12:00:00", None).unwrap();
    /// dt.add("5-1 0:0:30")
    /// assert_eq!(dt.to_timestamp(None), "2020-01-02 12:00:30");
    /// ```
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
                self._add_months(parsed.months as i32 - 12);
            } else {
                self._add_months(parsed.months as i32);
            }
        }

        // Add the rest of it as a single duration
        let dur = Duration::seconds(parsed.calc_duration());
        self.dt = self.dt + dur;
        Ok(())
    }

    /// Subtracts a partial time from the `DateTime` object. Partial times must be
    /// provided as strings in the general format %Y-%m-%d %H:%M:%S. The method
    /// is void, but on failure, it raises an error.
    /// 
    /// For the exact rules of partial time strings, see the `TimeFreq` documentation.
    /// 
    /// Wraps around years. For non-trivial behavior with months, see
    /// the documentation of `DateTime::_sub_months`.
    /// 
    /// ## Arguments
    /// * `timestamp` A partial time string
    /// 
    /// ## Examples
    /// ```
    /// let dt: DateTime = DateTime::from_timestamp("2019-01-01 12:00:00", None).unwrap();
    /// dt.subtract("3:15:30").unwrap();
    /// assert_eq!(dt.to_timestamp(None), "2019-01-01 08:44:30");
    /// 
    /// let dt: DateTime = DateTime::from_timestamp("2019-08-05 12:00:00", None).unwrap();
    /// dt.subtract("9-1 0:0:30")
    /// assert_eq!(dt.to_timestamp(None), "2018-12-04 11:59:30");
    /// ```
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
                // NOTE: Sign here must be negative or months must be added
                self._add_months(12 - parsed.months as i32);
            } else {
                self._sub_months(parsed.months as i32);
            }
        }

        // Subtract the rest of it as a single duration
        let dur = Duration::seconds(parsed.calc_duration());
        self.dt = self.dt - dur;
        Ok(())
    }

    /// Checks if the time represented by the `DateTime` object
    /// has passed relative to another `DateTime` object. If no
    /// reference is provided, the current time is used as a reference.
    /// 
    /// ## Arguments
    /// * `ref_dt` A reference `DateTime` object
    /// 
    /// ## Examples
    /// ```
    /// let dt: DateTime = DateTime::from_timestamp("2018-01-01 12:00:00", None).unwrap();
    /// let ref_dt: DateTime = DateTime::from_timestamp("2019-01-01 12:00:00", None).unwrap();
    /// dt.is_passed(Some(ref_dt); // false
    /// dt.is_passed(None); // true
    /// ```
    pub fn is_passed(&self, ref_dt: Option<&DateTime>) -> bool {
        if let Some(dt) = ref_dt {
            dt.dt > self.dt
        } else {
            Utc::now() > self.dt
        }
    }

    /// Calculates the next occurrence of a partial time string, and creates a
    /// `DateTime` object as a result. If it fails, it raises an error. Partial
    /// times must be provided as strings in the general format %Y-%m-%d %H:%M:%S.
    /// 
    /// For the exact rules of partial time strings, see the `TimeFreq` documentation.
    /// 
    /// Using it with days beyond 28, and with February 29 may cause unexpected results.
    /// For more information, see the documentation of `DateTime::_add_months`.
    /// 
    /// ## Arguments
    /// * `timestamp` A partial time string
    /// 
    /// ## Examples
    /// ```
    /// // It is 2019-07-26 12:00
    /// let dt: DateTime = DateTime::next_occurrence("15:00:00").unwrap();
    /// assert_eq!(dt.to_timestamp(None).unwrap(), "2019-07-26 15:00:00");
    /// 
    /// let dt: DateTime = DateTime::next_occurrence("10:00:00").unwrap();
    /// assert_eq!(dt.to_timestamp(None).unwrap(), "2019-07-27 10:00:00");
    /// 
    /// let dt: DateTime = DateTime::next_occurrence("1 15:00:00").unwrap();
    /// assert_eq!(dt.to_timestamp(None).unwrap(), "2019-08-01 15:00:00");
    /// ```
    pub fn next_occurrence(timestamp: &str) -> Result<DateTime, Box<Error>> {
        let dt = DateTime::now();

        DateTime::_next_occurrence(timestamp, &dt)
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

    mod _read_timezone {
        use super::super::*;

        #[test]
        fn reads_timezone() {
            let tz = DateTime::_read_timezone(Some("CET")).unwrap();
            let dt = tz.timestamp(1_500_000_000, 0);
            assert_eq!(dt.format("%z").to_string(), "+0200");
        }

        #[test]
        fn defaults_to_utc() {
            let tz = DateTime::_read_timezone(None).unwrap();
            let dt = tz.timestamp(1_500_000_000, 0);
            assert_eq!(dt.format("%z").to_string(), "+0000");
        }

        #[test]
        fn works_with_offsetted_gmt() {
            let mut tz = DateTime::_read_timezone(Some("GMT+2")).unwrap();
            let mut dt = tz.timestamp(1_500_000_000, 0);
            assert_eq!(dt.format("%z").to_string(), "-0200");

            tz = DateTime::_read_timezone(Some("Etc/GMT-2")).unwrap();
            dt = tz.timestamp(1_500_000_000, 0);
            assert_eq!(dt.format("%z").to_string(), "+0200");
        }

        #[test]
        fn invalid_tz_throws_error() {
            let tz = DateTime::_read_timezone(Some("Invalid"));
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
            assert!(!timeobj.is_passed(None));
        }

        #[test]
        fn returns_true_if_passed() {
            let timeobj = DateTime::from_timestamp("1972-02-02 22:22:22", None).unwrap();
            assert!(timeobj.is_passed(None));
        }
    }

    mod _merge_error {
        use super::super::*;

        #[test]
        fn returns_result_if_not_null() {
            let chrono_res = Utc::now().with_minute(40);
            let merge_res = DateTime::_merge_error(chrono_res, String::from("Some error message."));
            assert!(merge_res.is_ok())
        }

        #[test]
        fn returns_error_if_null() {
            let chrono_res = Utc::now().with_minute(70);
            let merge_res = DateTime::_merge_error(chrono_res, String::from("Some error message."));
            assert!(merge_res.is_err());

            let err = merge_res.err().unwrap();
            assert_eq!(*err.message, String::from("Some error message."));
        }
    }

    mod _merge_timefreq {
        use super::super::*;

        #[test]
        fn merges_seconds() {
            let old_now = Utc::now();
            let mut dt = DateTime::now();
            let mut tf = TimeFreq::from_timestamp("10", true).unwrap();
            dt._merge_timefreq(&mut tf).unwrap();

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
            dt._merge_timefreq(&mut tf).unwrap();

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
            dt._merge_timefreq(&mut tf).unwrap();

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
            dt._merge_timefreq(&mut tf).unwrap();

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
            dt._merge_timefreq(&mut tf).unwrap();

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
            dt._merge_timefreq(&mut tf).unwrap();

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
            let mut res = dt._merge_timefreq(&mut tf);
            assert!(res.is_err());

            //Day cannot be 0, only if it is not part of the relative date
            tf = TimeFreq::from_timestamp("0-1-0 0:0:0", true).unwrap();
            res = dt._merge_timefreq(&mut tf);
            assert!(res.is_err());

            //Month cannot be 0, only if it is not part of the relative date
            tf = TimeFreq::from_timestamp("1001-0-1 0:0:0", true).unwrap();
            res = dt._merge_timefreq(&mut tf);
            assert!(res.is_err());
        }
    }

    mod next_occurrence {
        use super::super::*;

        #[test]
        fn uses_current_time_as_reference() {
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

    mod _next_occurrence {
        use super::super::*;

        #[test]
        fn correct_in_common_cases() {
            let mut dt = DateTime::from_timestamp("2019-01-01 10:00:05", None).unwrap();
            let mut next_occur = DateTime::_next_occurrence("04", &dt).unwrap();
            assert_eq!(next_occur.to_timestamp(None).unwrap(), "2019-01-01 10:01:04");

            dt = DateTime::from_timestamp("2019-01-01 10:05:00", None).unwrap();
            next_occur = DateTime::_next_occurrence("04:00", &dt).unwrap();
            assert_eq!(next_occur.to_timestamp(None).unwrap(), "2019-01-01 11:04:00");

            dt = DateTime::from_timestamp("2019-01-01 10:00:00", None).unwrap();
            next_occur = DateTime::_next_occurrence("09:02:00", &dt).unwrap();
            assert_eq!(next_occur.to_timestamp(None).unwrap(), "2019-01-02 09:02:00");

            dt = DateTime::from_timestamp("2019-01-01 11:00:00", None).unwrap();
            next_occur = DateTime::_next_occurrence("10:02:00", &dt).unwrap();
            assert_eq!(next_occur.to_timestamp(None).unwrap(), "2019-01-02 10:02:00");

            dt = DateTime::from_timestamp("2019-01-01 10:00:00", None).unwrap();
            next_occur = DateTime::_next_occurrence("00:00:00", &dt).unwrap();
            assert_eq!(next_occur.to_timestamp(None).unwrap(), "2019-01-02 00:00:00");

            dt = DateTime::from_timestamp("2019-01-12 10:00:00", None).unwrap();
            next_occur = DateTime::_next_occurrence("07 00:00:00", &dt).unwrap();
            assert_eq!(next_occur.to_timestamp(None).unwrap(), "2019-02-07 00:00:00");

            dt = DateTime::from_timestamp("2019-03-12 10:00:00", None).unwrap();
            next_occur = DateTime::_next_occurrence("02-01 00:00:00", &dt).unwrap();
            assert_eq!(next_occur.to_timestamp(None).unwrap(), "2020-02-01 00:00:00");

            // Neccessary for defining one-shots in an unusual way
            dt = DateTime::from_timestamp("2019-03-12 10:00:00", None).unwrap();
            next_occur = DateTime::_next_occurrence("2020-01-01 00:00:00", &dt).unwrap();
            assert_eq!(next_occur.to_timestamp(None).unwrap(), "2020-01-01 00:00:00");
        }

        #[test]
        fn handles_edge_cases() {
            let mut dt = DateTime::from_timestamp("2019-01-31 10:00:00", None).unwrap();
            let mut next_occur = DateTime::_next_occurrence("31 00:00:00", &dt).unwrap();
            assert_eq!(next_occur.to_timestamp(None).unwrap(), "2019-03-03 00:00:00");

            dt = DateTime::from_timestamp("2019-07-31 12:00:00", None).unwrap();
            next_occur = DateTime::_next_occurrence("31 11:30:00", &dt).unwrap();
            assert_eq!(next_occur.to_timestamp(None).unwrap(), "2019-08-31 11:30:00");

            dt = DateTime::from_timestamp("2019-02-15 12:00:00", None).unwrap();
            next_occur = DateTime::_next_occurrence("30 11:30:00", &dt).unwrap();
            assert_eq!(next_occur.to_timestamp(None).unwrap(), "2019-03-02 11:30:00");

            dt = DateTime::from_timestamp("2019-01-01 12:00:00", None).unwrap();
            next_occur = DateTime::_next_occurrence("02-29 11:30:00", &dt).unwrap();
            assert_eq!(next_occur.to_timestamp(None).unwrap(), "2019-03-01 11:30:00");
        }
    }

    mod _add_months {
        use super::super::*;

        #[test]
        fn handles_normal_cases() {
            let mut timeobj = DateTime::from_timestamp("2018-08-15 10:30:00", None).unwrap();
            timeobj._add_months(1);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-09-15 10:30:00");

            timeobj._add_months(1);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-10-15 10:30:00");

            timeobj._add_months(2);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-12-15 10:30:00");
        }

        #[test]
        fn handles_edge_cases() {
            let mut timeobj = DateTime::from_timestamp("2018-03-31 10:30:00", None).unwrap();
            timeobj._add_months(1);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-05-01 10:30:00");

            timeobj = DateTime::from_timestamp("2018-01-31 10:30:00", None).unwrap();
            timeobj._add_months(1);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-03-03 10:30:00");

            timeobj = DateTime::from_timestamp("2018-01-30 10:30:00", None).unwrap();
            timeobj._add_months(1);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-03-02 10:30:00");

            timeobj = DateTime::from_timestamp("2018-01-29 10:30:00", None).unwrap();
            timeobj._add_months(1);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-03-01 10:30:00");
        }
    }

    mod _sub_months {
        use super::super::*;

        #[test]
        fn handles_normal_cases() {
            let mut timeobj = DateTime::from_timestamp("2018-12-15 10:30:00", None).unwrap();
            timeobj._sub_months(1);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-11-15 10:30:00");

            timeobj._sub_months(1);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-10-15 10:30:00");

            timeobj._sub_months(2);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-08-15 10:30:00");
        }

        #[test]
        fn handles_edge_cases() {
            let mut timeobj = DateTime::from_timestamp("2018-05-31 10:30:00", None).unwrap();
            timeobj._sub_months(1);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-04-30 10:30:00");

            timeobj = DateTime::from_timestamp("2018-03-31 10:30:00", None).unwrap();
            timeobj._sub_months(1);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-02-28 10:30:00");

            timeobj = DateTime::from_timestamp("2018-03-30 10:30:00", None).unwrap();
            timeobj._sub_months(1);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-02-27 10:30:00");

            timeobj = DateTime::from_timestamp("2018-03-29 10:30:00", None).unwrap();
            timeobj._sub_months(1);
            assert_eq!(timeobj.to_timestamp(None).unwrap(), "2018-02-26 10:30:00");
        }
    }
}
