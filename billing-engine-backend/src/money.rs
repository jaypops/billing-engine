use serde::{Deserialize, Serialize};
use sqlx::Type;
use utoipa::ToSchema;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, sqlx::Type)]
#[serde(try_from = "i64")]
#[sqlx(transparent)]
pub struct Money(i64);

impl Money {
    pub fn new(kobo: i64) -> Result<Money, AppError> {
        if kobo <= 0 {
            return Err(AppError::Validation(
                "Money value must be greater than zero".into(),
            ));
        }
        Ok(Money(kobo))
    }
    pub fn kobo(&self) -> i64 {
        self.0
    }
    pub fn checked_add(self, other: Money) -> Option<Money> {
        self.0.checked_add(other.0).map(Money)
    }
}

impl TryFrom<i64> for Money {
    type Error = AppError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Money::new(value)
    }
}
