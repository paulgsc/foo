use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use sqlx::encode::IsNull;
use sqlx::sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef};
use sqlx::{Decode, Encode, Sqlite, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SqliteDateTime(pub DateTime<Utc>);

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
}

impl Encode<'_, Sqlite> for SqliteDateTime {
	fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'_>>) -> IsNull {
		<i64 as Encode<Sqlite>>::encode_by_ref(&self.0.timestamp(), args)
	}
}

impl<'r> Decode<'r, Sqlite> for SqliteDateTime {
	fn decode(value: SqliteValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		let timestamp = <i64 as Decode<Sqlite>>::decode(value)?;
		let datetime = Utc.timestamp_opt(timestamp, 0).single().ok_or("Invalid timestamp")?;
		Ok(Self(datetime))
	}
}
