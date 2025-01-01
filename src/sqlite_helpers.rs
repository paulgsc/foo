use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use serde_json::Value;
use sqlite_macros::SqliteType;
use std::fmt;
use std::str::FromStr;

pub trait SqliteValidate {
	type Error;
	fn validate(s: &str) -> Result<(), Self::Error>;
}

// Use the derived macro for SqliteDateTime
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, SqliteType)]
#[sqlite_type(validate, max_length = "20")]
pub struct SqliteDateTime(pub DateTime<Utc>);

impl fmt::Display for SqliteDateTime {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.timestamp())
	}
}

impl SqliteDateTime {
	pub(crate) fn now() -> Self {
		Self(Utc::now())
	}

	pub(crate) fn timestamp(&self) -> i64 {
		self.0.timestamp()
	}
}

impl FromStr for SqliteDateTime {
	type Err = sqlx::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let timestamp = s.parse::<i64>().map_err(|_| sqlx::Error::Protocol("Invalid timestamp format".into()))?;

		if timestamp < 0 {
			return Err(sqlx::Error::Protocol("Timestamp cannot be negative".into()));
		}

		let datetime = Utc.timestamp_opt(timestamp, 0).single().ok_or_else(|| sqlx::Error::Protocol("Invalid timestamp".into()))?;

		Ok(Self(datetime))
	}
}

impl From<i64> for SqliteDateTime {
	fn from(timestamp: i64) -> Self {
		let datetime = Utc.timestamp_opt(timestamp, 0).single().unwrap();
		Self(datetime)
	}
}

impl std::ops::Add<TimeDelta> for SqliteDateTime {
	type Output = Self;

	fn add(self, rhs: TimeDelta) -> Self::Output {
		Self(self.0 + rhs)
	}
}

impl SqliteValidate for SqliteDateTime {
	type Error = sqlx::Error;

	fn validate(s: &str) -> Result<(), Self::Error> {
		match s.parse::<i64>() {
			Ok(timestamp) => {
				if timestamp < 0 {
					return Err(sqlx::Error::Protocol("Timestamp cannot be negative".into()));
				}
				match Utc.timestamp_opt(timestamp, 0).single() {
					Some(_) => Ok(()),
					None => Err(sqlx::Error::Protocol("Invalid timestamp".into())),
				}
			}
			Err(_) => Err(sqlx::Error::Protocol("Invalid timestamp format".into())),
		}
	}
}

// Use the derived macro for OptionalSqliteDateTime
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, SqliteType)]
pub struct OptionalSqliteDateTime(pub Option<SqliteDateTime>);

impl fmt::Display for OptionalSqliteDateTime {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.0 {
			Some(dt) => write!(f, "{}", dt),
			None => write!(f, "NULL"),
		}
	}
}

impl SqliteValidate for OptionalSqliteDateTime {
	type Error = sqlx::Error;

	fn validate(s: &str) -> Result<(), Self::Error> {
		SqliteDateTime::validate(s)
	}
}

impl FromStr for OptionalSqliteDateTime {
	type Err = sqlx::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s.is_empty() {
			Ok(Self(None))
		} else {
			SqliteDateTime::from_str(s).map(|dt| Self(Some(dt)))
		}
	}
}

impl From<i64> for OptionalSqliteDateTime {
	fn from(timestamp: i64) -> Self {
		Self(Some(SqliteDateTime::from(timestamp)))
	}
}

impl From<i32> for OptionalSqliteDateTime {
	fn from(timestamp: i32) -> Self {
		Self(Some(SqliteDateTime::from(timestamp as i64)))
	}
}

impl From<Option<i64>> for OptionalSqliteDateTime {
	fn from(opt: Option<i64>) -> Self {
		Self(opt.map(SqliteDateTime::from))
	}
}

impl From<Option<i32>> for OptionalSqliteDateTime {
	fn from(opt: Option<i32>) -> Self {
		Self(opt.map(|ts| SqliteDateTime::from(ts as i64)))
	}
}

// Use the derived macro for OptionalJsonValue
#[derive(Debug, Clone, PartialEq, Eq, SqliteType)]
pub struct OptionalJsonValue(pub Option<Value>);

impl fmt::Display for OptionalJsonValue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match &self.0 {
			Some(value) => match serde_json::to_string(value) {
				Ok(json_str) => write!(f, "{}", json_str),
				Err(_) => write!(f, "null"),
			},
			None => write!(f, "null"),
		}
	}
}

impl SqliteValidate for OptionalJsonValue {
	type Error = sqlx::Error;

	fn validate(s: &str) -> Result<(), Self::Error> {
		Self::from_str(s).map(|_| ())
	}
}

impl FromStr for OptionalJsonValue {
	type Err = sqlx::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match serde_json::from_str(s) {
			Ok(value) => Ok(OptionalJsonValue(Some(value))),
			Err(e) => Err(sqlx::Error::Protocol(e.to_string())),
		}
	}
}

impl From<String> for OptionalJsonValue {
	fn from(s: String) -> Self {
		let value: Value = serde_json::from_str(&s).unwrap_or(Value::Null);
		OptionalJsonValue(Some(value))
	}
}

impl From<Option<String>> for OptionalJsonValue {
	fn from(s: Option<String>) -> Self {
		match s {
			Some(value) => OptionalJsonValue::from(value),
			None => OptionalJsonValue(None),
		}
	}
}
