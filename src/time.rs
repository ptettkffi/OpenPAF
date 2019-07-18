use chrono::{TimeZone, Utc};
use chrono::DateTime as ChronoDateTime;
use chrono_tz::Tz;

const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

pub struct DateTime {
    dt: ChronoDateTime<Utc>
}

impl DateTime {
    pub fn from_timestamp(ts: &str, timezone: Option<&str>) -> DateTime {
        let tz_str = timezone.unwrap_or("UTC");
        let tz: Tz = tz_str.parse().unwrap();
        let dt = tz.datetime_from_str(ts, TIMESTAMP_FORMAT).unwrap().with_timezone(&Utc);
        DateTime {dt: dt}
    }

    pub fn from_epoch(epoch: i64) -> DateTime {
        DateTime {dt: Utc.timestamp(epoch, 0)}
    }

    pub fn now() -> DateTime {
        DateTime {dt: Utc::now()}
    }

    pub fn to_timestamp(&self, timezone: Option<&str>) -> String {
        let tz_str = timezone.unwrap_or("UTC");
        let tz: Tz = tz_str.parse().unwrap();
        self.dt.with_timezone(&tz).format(TIMESTAMP_FORMAT).to_string()
    }

    pub fn to_epoch(&self) -> i64 {
        self.dt.timestamp()
    }

}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn reads_from_epoch() {
        let timeobj = DateTime::from_epoch(1_500_000_000);
        assert_eq!(timeobj.to_epoch(), 1_500_000_000);
    }

    #[test]
    fn reads_from_timestamp() {
        let timeobj = DateTime::from_timestamp("2017-07-14T02:40:00", None);
        assert_eq!(timeobj.to_epoch(), 1_500_000_000);
    }

    #[test]
    fn creates_from_current_time() {
        let timeobj = DateTime::now();
        assert!(timeobj.to_epoch() == Utc::now().timestamp())
    }

    #[test]
    fn converts_to_timestamp() {
        let timeobj = DateTime::from_epoch(1_500_000_000);
        let timestamp = timeobj.to_timestamp(None);
        assert_eq!(timestamp, "2017-07-14T02:40:00");
    }

    #[test]
    fn handles_timezones_in_timestamp() {
        let timeobj = DateTime::from_timestamp("2017-07-14T02:40:00", Some("CET"));
        let timestamp = timeobj.to_timestamp(None);
        assert_eq!(timestamp, "2017-07-14T00:40:00");
    }

    #[test]
    fn handles_daylight_savings() {
        let timeobj = DateTime::from_timestamp("2017-03-14T02:40:00", Some("CET"));
        let timestamp = timeobj.to_timestamp(None);
        assert_eq!(timestamp, "2017-03-14T01:40:00");
    }

    #[test]
    fn converts_timestamp_to_timezone() {
        let timeobj = DateTime::from_epoch(1_500_000_000);
        let timestamp = timeobj.to_timestamp(Some("CET"));
        assert_eq!(timestamp, "2017-07-14T04:40:00");
    }

}
