use std::error::Error;
use super::super::error::PafError;

enum DateOrTime {
    Date,
    Time
}

// Struct for storing relative times and frequencies
// (e.g. 15:00 translates to every 15 hours in frequency and 15:00:00 every day in time)
// used for scheduling.
pub struct TimeFreq {
    // Time/frequency components
    // NOTE: chrono::DateTime uses i32 for years, as it needs to handle BC times. We neglect them as
    // they are not important for the application, and signed years can introduce unnecessary complexity
    // to frequencies.
    pub years: u32,
    pub months: u32,
    pub days: u32,
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32
}

impl Default for TimeFreq {
    fn default() -> TimeFreq {
        TimeFreq {
            years: 0,
            months: 0,
            days: 0,
            hours: 0,
            minutes: 0,
            seconds: 0
        }
    }
}

impl TimeFreq {
    fn sanitize_timestr_arr(str_arr: &mut Vec<&str>) {
        // Remove empty elements
        str_arr.retain(|e| !e.trim().is_empty());
    }

    fn parse_timestamp(timestamp: &str, dot: DateOrTime) -> Result<Vec<u32>, Box<Error>> {
        // Prepare some variables
        let separator = match dot {
            DateOrTime::Date => "-",
            DateOrTime::Time => ":"
        };

        // Process string
        let mut timestamp_arr: Vec<u32> = Vec::with_capacity(3);
        let mut timestamp_str_arr: Vec<&str> = timestamp.trim().split(separator).collect();
        TimeFreq::sanitize_timestr_arr(&mut timestamp_str_arr);

        // Throw error, if obviously invalid
        if timestamp_str_arr.is_empty() || timestamp_str_arr.len() > 3 {
            return Err(PafError::create_error("Invalid timestamp."));
        }

        // Try to parse elements
        for i in 0..timestamp_str_arr.len() {
            let val: u32 = timestamp_str_arr[i].trim().parse()?;
            timestamp_arr.push(val);
        }

        for _ in 0..3 - timestamp_arr.len() {
            match dot {
                DateOrTime::Time => timestamp_arr.push(0),
                DateOrTime::Date => timestamp_arr.insert(0, 0)
            }
        }

        Ok(timestamp_arr)
    }

    pub fn from_timestamp(timestamp: &str) -> Result<TimeFreq, Box<Error>> {
        let mut date_arr: Vec<u32> = vec![0, 0, 0];
        let mut time_arr: Vec<u32> = vec![0, 0, 0];

        // Process input string
        let mut ts_arr: Vec<&str> = timestamp.trim().split(" ").collect();

        // Try to correct bad formatting
        TimeFreq::sanitize_timestr_arr(&mut ts_arr);

        // Check if we have an empty or invalid input
        if ts_arr.is_empty() {
            return Err(PafError::create_error("Failed to parse empty timestamp."));
        } else if ts_arr.len() > 2 {
            return Err(PafError::create_error("Failed to parse invalid timestamp."));
        }

        // If we have one element, decide if it's a date or a time
        if ts_arr.len() == 1 {
            if ts_arr[0].contains(":") && ts_arr[0].contains("-") {
                return Err(PafError::create_error("Invalid timestamp."));
            }
            else if ts_arr[0].contains("-") {
                date_arr = TimeFreq::parse_timestamp(ts_arr[0], DateOrTime::Date)?;
            } else {
                time_arr = TimeFreq::parse_timestamp(ts_arr[0], DateOrTime::Time)?;
            }
        } else {
            date_arr = TimeFreq::parse_timestamp(ts_arr[0], DateOrTime::Date)?;
            time_arr = TimeFreq::parse_timestamp(ts_arr[1], DateOrTime::Time)?;
        }

        // Convert excess months to years
        if date_arr[1] >= 12 {
            let years_f: f32 = date_arr[1] as f32 / 12.0;
            let years = years_f.floor() as u32;

            date_arr[0] += years;
            date_arr[1] -= years * 12;
        }

        Ok(TimeFreq {
            years: date_arr[0],
            months: date_arr[1],
            days: date_arr[2],
            hours: time_arr[0],
            minutes: time_arr[1],
            seconds: time_arr[2]
        })
    }

    pub fn from_epoch(epoch: u32) -> TimeFreq {
        TimeFreq{seconds: epoch, ..Default::default()}
    }

    pub fn calc_duration(&self) -> i64 {
        let mut secs: i64 = self.days as i64 * 24 * 60 * 60;
        secs += self.hours as i64 * 60 * 60;
        secs += self.minutes as i64 * 60;
        secs + self.seconds as i64
    }
}

#[cfg(test)]
mod tests {
    mod sanitize_timestr_arr {
        use super::super::*;

        #[test]
        fn returns_correct_array() {
            let mut timearr = "1-1-1".split("-").collect();
            TimeFreq::sanitize_timestr_arr(&mut timearr);
            assert_eq!(timearr, ["1", "1", "1"]);
        }

        #[test]
        fn removes_empty_members() {
            let mut timearr = "\n    1-1-1   1:1:1 \t".split(" ").collect();
            TimeFreq::sanitize_timestr_arr(&mut timearr);
            assert_eq!(timearr, ["1-1-1", "1:1:1"]);
        }
    }

    mod parse_timestamp {
        use super::super::*;

        #[test]
        fn parses_full_date() {
            let datearr = TimeFreq::parse_timestamp("1-2-3", DateOrTime::Date).unwrap();
            assert_eq!(datearr, [1, 2, 3]);
        }

        #[test]
        fn parses_partial_date() {
            let mut datearr = TimeFreq::parse_timestamp("1-2", DateOrTime::Date).unwrap();
            assert_eq!(datearr, [0, 1, 2]);

            datearr = TimeFreq::parse_timestamp("1", DateOrTime::Date).unwrap();
            assert_eq!(datearr, [0, 0, 1]);
        }

        #[test]
        fn parses_full_time() {
            let datearr = TimeFreq::parse_timestamp("1:2:3", DateOrTime::Time).unwrap();
            assert_eq!(datearr, [1, 2, 3]);
        }

        #[test]
        fn parses_partial_time() {
            let mut datearr = TimeFreq::parse_timestamp("1:2", DateOrTime::Time).unwrap();
            assert_eq!(datearr, [1, 2, 0]);

            datearr = TimeFreq::parse_timestamp("1", DateOrTime::Time).unwrap();
            assert_eq!(datearr, [1, 0, 0]);
        }
    }

    mod from_timestamp {
        use super::super::*;

        #[test]
        fn parses_full_timestamp() {
            let timestamp = "1-2-3 4:5:6";
            let ts_obj = TimeFreq::from_timestamp(timestamp).unwrap();
            assert_eq!(1, ts_obj.years);
            assert_eq!(2, ts_obj.months);
            assert_eq!(3, ts_obj.days);
            assert_eq!(4, ts_obj.hours);
            assert_eq!(5, ts_obj.minutes);
            assert_eq!(6, ts_obj.seconds);
        }

        #[test]
        fn parses_partial_timestamp() {
            let timestamp = "2-3 4:5";
            let ts_obj = TimeFreq::from_timestamp(timestamp).unwrap();
            assert_eq!(0, ts_obj.years);
            assert_eq!(2, ts_obj.months);
            assert_eq!(3, ts_obj.days);
            assert_eq!(4, ts_obj.hours);
            assert_eq!(5, ts_obj.minutes);
            assert_eq!(0, ts_obj.seconds);
        }

        #[test]
        fn parses_md() {
            let timestamp = "2-3";
            let ts_obj = TimeFreq::from_timestamp(timestamp).unwrap();
            assert_eq!(0, ts_obj.years);
            assert_eq!(2, ts_obj.months);
            assert_eq!(3, ts_obj.days);
            assert_eq!(0, ts_obj.hours);
            assert_eq!(0, ts_obj.minutes);
            assert_eq!(0, ts_obj.seconds);
        }

        #[test]
        fn parses_hm() {
            let timestamp = "2:3";
            let ts_obj = TimeFreq::from_timestamp(timestamp).unwrap();
            assert_eq!(0, ts_obj.years);
            assert_eq!(0, ts_obj.months);
            assert_eq!(0, ts_obj.days);
            assert_eq!(2, ts_obj.hours);
            assert_eq!(3, ts_obj.minutes);
            assert_eq!(0, ts_obj.seconds);
        }

        #[test]
        fn handles_zeroes() {
            let timestamp = "1-0-0 0:0:5";
            let ts_obj = TimeFreq::from_timestamp(timestamp).unwrap();
            assert_eq!(1, ts_obj.years);
            assert_eq!(0, ts_obj.months);
            assert_eq!(0, ts_obj.days);
            assert_eq!(0, ts_obj.hours);
            assert_eq!(0, ts_obj.minutes);
            assert_eq!(5, ts_obj.seconds);
        }

        #[test]
        fn handles_zeroes_in_date() {
            let mut timestamp = "1-0-0";
            let mut ts_obj = TimeFreq::from_timestamp(timestamp).unwrap();
            assert_eq!(1, ts_obj.years);
            assert_eq!(0, ts_obj.months);
            assert_eq!(0, ts_obj.days);
            assert_eq!(0, ts_obj.hours);
            assert_eq!(0, ts_obj.minutes);
            assert_eq!(0, ts_obj.seconds);

            timestamp = "11-0";
            ts_obj = TimeFreq::from_timestamp(timestamp).unwrap();
            assert_eq!(0, ts_obj.years);
            assert_eq!(11, ts_obj.months);
            assert_eq!(0, ts_obj.days);
            assert_eq!(0, ts_obj.hours);
            assert_eq!(0, ts_obj.minutes);
            assert_eq!(0, ts_obj.seconds);
        }

        #[test]
        fn handles_zeroes_in_time() {
            let timestamp = "0:0:1";
            let ts_obj = TimeFreq::from_timestamp(timestamp).unwrap();
            assert_eq!(0, ts_obj.years);
            assert_eq!(0, ts_obj.months);
            assert_eq!(0, ts_obj.days);
            assert_eq!(0, ts_obj.hours);
            assert_eq!(0, ts_obj.minutes);
            assert_eq!(1, ts_obj.seconds);
        }

        #[test]
        fn parses_edge_case_timestamp() {
            let timestamp = "1 2";
            let ts_obj = TimeFreq::from_timestamp(timestamp).unwrap();
            assert_eq!(0, ts_obj.years);
            assert_eq!(0, ts_obj.months);
            assert_eq!(1, ts_obj.days);
            assert_eq!(2, ts_obj.hours);
            assert_eq!(0, ts_obj.minutes);
            assert_eq!(0, ts_obj.seconds);
        }

        #[test]
        fn throws_error_on_empty_string() {
            let ts_obj = TimeFreq::from_timestamp("");
            assert!(ts_obj.is_err());
        }

        #[test]
        fn throws_error_on_ambiguous_ts() {
            let mut ts_obj = TimeFreq::from_timestamp("1-2-3 4:5:6 7-8-9 10:11:12");
            assert!(ts_obj.is_err());

            ts_obj = TimeFreq::from_timestamp("1-2-3-4 5:6:7");
            assert!(ts_obj.is_err());

            ts_obj = TimeFreq::from_timestamp("1-2-3 4:5:6:7");
            assert!(ts_obj.is_err());
        }

        #[test]
        fn throws_error_on_invalid_ts() {
            let mut ts_obj = TimeFreq::from_timestamp("One-2-3 4:5:6");
            assert!(ts_obj.is_err());

            ts_obj = TimeFreq::from_timestamp("1-2-3 Five:6:7");
            assert!(ts_obj.is_err());

            ts_obj = TimeFreq::from_timestamp("a-b-c d:e:f");
            assert!(ts_obj.is_err());
        }

        #[test]
        fn tolerates_bad_formatting() {
            let ts_obj = TimeFreq::from_timestamp("   1-2-3  \n  4:5:6  \t");
            assert!(!ts_obj.is_err());
        }

        #[test]
        fn wraps_years() {
            let mut ts_obj = TimeFreq::from_timestamp("12-0").unwrap();
            assert_eq!(1, ts_obj.years);
            assert_eq!(0, ts_obj.months);

            ts_obj = TimeFreq::from_timestamp("30-0").unwrap();
            assert_eq!(2, ts_obj.years);
            assert_eq!(6, ts_obj.months);
        }
    }
}
