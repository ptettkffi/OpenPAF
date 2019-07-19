use chrono::{TimeZone, Utc};
use chrono::DateTime as ChronoDateTime;
use chrono_tz::Tz;
use std::error::Error;

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
        let dt = tz.datetime_from_str(ts, TIMESTAMP_FORMAT).unwrap().with_timezone(&Utc);
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
}
