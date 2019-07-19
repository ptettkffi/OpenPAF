use std::error::Error;
use super::super::error::PafError;

enum DateOrTime {
    Date,
    Time
}

pub struct TimeParser {
    years: i64,
    months: i64,
    days: i64,
    hours: i64,
    minutes: i64,
    seconds: i64
}

impl Default for TimeParser {
    fn default() -> TimeParser {
        TimeParser {
            years: 0,
            months: 0,
            days: 0,
            hours: 0,
            minutes: 0,
            seconds: 0
        }
    }
}

impl TimeParser {
    fn sanitize_timestring(str_arr: &mut Vec<&str>) {
        // Remove empty elements
        str_arr.retain(|e| !e.trim().is_empty());
    }

    fn parse_timestamp(timestamp: &str, dot: DateOrTime) -> Result<Vec<i64>, Box<Error>> {
        // Prepare some variables
        let separator = match dot {
            DateOrTime::Date => "-",
            DateOrTime::Time => ":"
        };

        // Process string
        let mut timestamp_arr: Vec<i64> = vec![0, 0, 0];
        let mut timestamp_str_arr: Vec<&str> = timestamp.trim().split(separator).collect();
        TimeParser::sanitize_timestring(&mut timestamp_str_arr);

        // Throw error, if obviously invalid
        if timestamp_str_arr.is_empty() || timestamp_str_arr.len() > 3 {
            return Err(PafError::create_error("Invalid timestamp."));
        }

        // Try to parse elements
        for i in 0..timestamp_str_arr.len() {
            let val: i64 = timestamp_str_arr[i].trim().parse()?;
            match dot {
                DateOrTime::Date => timestamp_arr[2 - i] = val,
                DateOrTime::Time => timestamp_arr[i] = val
            }
        }
        Ok(timestamp_arr)
    }

    pub fn from_timestamp(timestamp: &str) -> Result<TimeParser, Box<Error>> {
        let mut date_arr: Vec<i64> = vec![0, 0, 0];
        let mut time_arr: Vec<i64> = vec![0, 0, 0];

        // Process input string
        let mut ts_arr: Vec<&str> = timestamp.trim().split(" ").collect();

        // Try to correct bad formatting
        TimeParser::sanitize_timestring(&mut ts_arr);

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
                date_arr = TimeParser::parse_timestamp(ts_arr[0], DateOrTime::Date)?;
            } else {
                time_arr = TimeParser::parse_timestamp(ts_arr[0], DateOrTime::Time)?;
            }
        } else {
            date_arr = TimeParser::parse_timestamp(ts_arr[0], DateOrTime::Date)?;
            time_arr = TimeParser::parse_timestamp(ts_arr[1], DateOrTime::Time)?;
        }

        Ok(TimeParser {
            years: date_arr[0],
            months: date_arr[1],
            days: date_arr[2],
            hours: time_arr[0],
            minutes: time_arr[1],
            seconds: time_arr[2]
        })
    }

    pub fn from_epoch(epoch: i64) -> TimeParser {
        TimeParser{seconds: epoch, ..Default::default()}
    }
}

#[cfg(test)]
mod tests {
    mod sanitize_timestring {

    }

    mod parse_timestamp {
        #[test]
        fn parses_full_date() {
            use super::super::*;

            let datearr = TimeParser::parse_timestamp("1-1-1", DateOrTime::Date).unwrap();
            assert_eq!(datearr, [1, 1, 1]);
        }

        #[test]
        fn parses_partial_date() {
            use super::super::*;

            let mut datearr = TimeParser::parse_timestamp("1-1", DateOrTime::Date).unwrap();
            assert_eq!(datearr, [0, 1, 1]);

            datearr = TimeParser::parse_timestamp("1", DateOrTime::Date).unwrap();
            assert_eq!(datearr, [0, 0, 1]);
        }

        #[test]
        fn parses_full_time() {
            use super::super::*;

            let datearr = TimeParser::parse_timestamp("1:1:1", DateOrTime::Time).unwrap();
            assert_eq!(datearr, [1, 1, 1]);
        }

        #[test]
        fn parses_partial_time() {
            use super::super::*;

            let mut datearr = TimeParser::parse_timestamp("1:1", DateOrTime::Time).unwrap();
            assert_eq!(datearr, [1, 1, 0]);

            datearr = TimeParser::parse_timestamp("1", DateOrTime::Time).unwrap();
            assert_eq!(datearr, [1, 0, 0]);
        }
    }
}
