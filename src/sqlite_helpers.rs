use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use serde_json::Value;
use sqlx::encode::IsNull;
use sqlx::sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef};
use sqlx::{Decode, Encode, Sqlite, Type, TypeInfo, ValueRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SqliteDateTime(pub DateTime<Utc>);

impl SqliteDateTime {
	pub(crate) fn now() -> Self {
		Self(Utc::now())
	}

	pub(crate) fn timestamp(&self) -> i64 {
		self.0.timestamp()
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

impl Type<Sqlite> for SqliteDateTime {
	fn type_info() -> SqliteTypeInfo {
		<i64 as Type<Sqlite>>::type_info()
	}

	fn compatible(ty: &SqliteTypeInfo) -> bool {
		*ty == <i64 as Type<Sqlite>>::type_info() || ty.name().to_lowercase().contains("datetime") || ty.name().to_lowercase().contains("timestamp")
	}
}

impl Encode<'_, Sqlite> for SqliteDateTime {
	fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'_>>) -> IsNull {
		args.push(SqliteArgumentValue::Int64(self.timestamp()));
		IsNull::No
	}
}

impl<'r> Decode<'r, Sqlite> for SqliteDateTime {
	fn decode(value: SqliteValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let timestamp = match value.type_info().name().to_lowercase().as_str() {
			"datetime" | "timestamp" => value.int64()?,
			_ => return Err("Unexpected type for datetime column".into()),
		};

		let datetime = Utc.timestamp_opt(timestamp, 0).single().ok_or("Invalid timestamp")?;

		Ok(Self(datetime))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OptionalSqliteDateTime(pub Option<SqliteDateTime>);

impl Type<Sqlite> for OptionalSqliteDateTime {
	fn type_info() -> SqliteTypeInfo {
		<i64 as Type<Sqlite>>::type_info()
	}

	fn compatible(ty: &SqliteTypeInfo) -> bool {
		*ty == <i64 as Type<Sqlite>>::type_info() || ty.name().to_lowercase().contains("datetime") || ty.name().to_lowercase().contains("timestamp")
	}
}

impl Encode<'_, Sqlite> for OptionalSqliteDateTime {
	fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'_>>) -> IsNull {
		match self.0 {
			Some(ref dt) => dt.encode_by_ref(args),
			None => IsNull::Yes,
		}
	}
}

impl<'r> Decode<'r, Sqlite> for OptionalSqliteDateTime {
	fn decode(value: SqliteValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let timestamp = match value.type_info().name().to_lowercase().as_str() {
			"datetime" | "timestamp" => value.int64().ok(),
			_ => None,
		};

		let datetime = timestamp.and_then(|ts| Utc.timestamp_opt(ts, 0).single());

		Ok(Self(datetime.map(SqliteDateTime)))
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionalJsonValue(pub Option<Value>);

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

impl Type<Sqlite> for OptionalJsonValue {
	fn type_info() -> SqliteTypeInfo {
		<&str as Type<Sqlite>>::type_info()
	}
}

impl Encode<'_, Sqlite> for OptionalJsonValue {
	fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'_>>) -> IsNull {
		match &self.0 {
			Some(value) => {
				let json_str = value.to_string();
				args.push(SqliteArgumentValue::Text(Box::leak(json_str.into_boxed_str())));
				IsNull::No
			}
			None => IsNull::Yes,
		}
	}
}

impl<'r> Decode<'r, Sqlite> for OptionalJsonValue {
	fn decode(value: SqliteValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		if value.is_null() {
			return Ok(Self(None));
		}

		let text = value.text()?;
		let json_value = serde_json::from_str(text)?;
		Ok(Self(Some(json_value)))
	}
}
