use chrono::{Duration, NaiveDateTime};
use serde::Deserialize;
use sqlx;
use sqlx::types::Uuid;
use std::mem;

pub mod unix_timestamp {
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let time: String = Deserialize::deserialize(deserializer)?;
        let dt = NaiveDateTime::parse_from_str(&time, "%Y-%m-%d %H:%M:%S")
            .map_err(serde::de::Error::custom)?;
        Ok(dt)
    }
}

#[derive(sqlx::FromRow)]
pub struct SecuritiesStr(pub String);

impl Into<String> for SecuritiesStr {
    fn into(self) -> String {
        self.0
    }
}

pub enum Frame {
    M1,
    H1,
    D1,
}

impl From<&str> for Frame {
    fn from(value: &str) -> Self {
        match value {
            "h1" => Self::H1,
            "d1" => Self::D1,
            "m1" => Self::M1,
            _ => unimplemented!("from {} to [Frame] not implemented", value),
        }
    }
}

impl ToString for Frame {
    fn to_string(&self) -> String {
        match self {
            Frame::M1 => String::from("m1"),
            Frame::H1 => String::from("h1"),
            Frame::D1 => String::from("d1"),
        }
    }
}

#[derive(Debug, Deserialize, sqlx::FromRow)]
#[allow(dead_code)]
pub struct Candle {
    pub open: f32,
    pub close: f32,
    pub high: f32,
    pub low: f32,
    pub value: f32,
    pub volume: f32,
    #[serde(with = "unix_timestamp")]
    pub begin: NaiveDateTime,
    #[serde(with = "unix_timestamp")]
    pub end: NaiveDateTime,
}

pub trait ToSql {
    fn for_insert(&self) -> String;
}

impl ToSql for Candle {
    fn for_insert(&self) -> String {
        format!(
            "{}, {}, {}, {}, {}, {}, '{}', '{}'",
            self.open,
            self.close,
            self.high,
            self.low,
            self.value,
            self.volume,
            self.begin,
            self.end
        )
    }
}

pub struct DateRange(pub NaiveDateTime, pub NaiveDateTime);

impl Iterator for DateRange {
    type Item = NaiveDateTime;
    fn next(&mut self) -> Option<Self::Item> {
        if self.0 <= self.1 {
            let next = self.0 + Duration::days(1);
            Some(mem::replace(&mut self.0, next))
        } else {
            None
        }
    }
}

pub enum OperationType {
    Purchase,
    Sale,
}

impl From<&str> for OperationType {
    fn from(value: &str) -> Self {
        match value {
            "purchase" => Self::Purchase,
            "sale" => Self::Sale,
            _ => unimplemented!("operation type: {} not implemented", value),
        }
    }
}

impl ToString for OperationType {
    fn to_string(&self) -> String {
        match self {
            OperationType::Purchase => String::from("purchase"),
            OperationType::Sale => String::from("sale"),
        }
    }
}

#[allow(dead_code)]
pub struct Operation {
    pub id: Uuid,
    pub attempt: Uuid,
    pub operation_type: OperationType,
    pub security: String,
    pub count: i32,
    pub price: f32,
    pub commission: f32,
    pub time_at: NaiveDateTime,
    pub sum_before: f32,
    pub sum_after: f32,
}
