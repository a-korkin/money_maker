use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
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

#[derive(Debug)]
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

#[derive(Debug, Deserialize, sqlx::FromRow, Clone)]
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
    #[sqlx(skip)]
    pub position_x: f32,
    #[sqlx(skip)]
    pub position_y: f32,
}

impl std::fmt::Display for Candle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let color = match (self.open, self.close) {
            (o, c) if o > c => "31",
            (o, c) if o < c => "32",
            _ => "37",
        };
        write!(
            f,
            "\x1b[{color}m{}\topen: {:.2}\tclose: {:.2}\x1b[0m",
            self.begin, self.open, self.close
        )
    }
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

#[derive(Debug, Deserialize, sqlx::FromRow)]
pub struct Trade {
    #[serde(rename = "TRADENO")]
    pub trade_no: i64,
    #[serde(rename = "TRADETIME")]
    pub trade_time: NaiveTime,
    #[serde(rename = "BOARDID")]
    pub board_id: String,
    #[serde(rename = "SECID")]
    pub sec_id: String,
    #[serde(rename = "PRICE")]
    pub price: f32,
    #[serde(rename = "QUANTITY")]
    pub quantity: i32,
    #[serde(rename = "VALUE")]
    pub value: f32,
    #[serde(rename = "PERIOD")]
    pub period: i8,
    #[serde(rename = "TRADETIME_GRP")]
    pub tradetime_grp: i32,
    #[serde(with = "unix_timestamp", rename = "SYSTIME")]
    pub systime: NaiveDateTime,
    #[serde(rename = "BUYSELL")]
    pub buysell: i8,
    #[serde(rename = "DECIMALS")]
    pub decimals: i32,
    #[serde(rename = "TRADINGSESSION")]
    pub trading_session: i32,
    #[serde(rename = "TRADEDATE")]
    pub trade_date: NaiveDate,
    #[serde(rename = "TRADE_SESSION_DATE")]
    pub trade_session_date: NaiveDate,
}

impl ToSql for Trade {
    fn for_insert(&self) -> String {
        format!(
            "{}, '{} {}', {}, {}, {}, '{}'",
            self.trade_no,
            self.trade_date,
            self.trade_time,
            self.price,
            self.quantity,
            self.value,
            self.buysell,
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
pub struct TradeView {
    pub trade_period: NaiveDateTime,
    pub buysell: String,
    pub price_all: f32,
    pub quantity_all: i64,
    pub value_all: f32,
    pub price_buy: f32,
    pub quantity_buy: i64,
    pub value_buy: f32,
    pub price_sell: f32,
    pub quantity_sell: i64,
    pub value_sell: f32,
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

#[allow(dead_code)]
pub enum AvgPeriod {
    Year,
    Month,
}

pub enum OperationType {
    Buy,
    Sold,
}

impl From<&str> for OperationType {
    fn from(value: &str) -> Self {
        match value {
            "buy" => Self::Buy,
            "sold" => Self::Sold,
            _ => unimplemented!("operation type: {} not implemented", value),
        }
    }
}

impl ToString for OperationType {
    fn to_string(&self) -> String {
        match self {
            OperationType::Buy => String::from("buy"),
            OperationType::Sold => String::from("sold"),
        }
    }
}

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

pub struct Attempt {
    pub id: Uuid,
    pub profit: f32,
    pub commission: f32,
}

#[allow(dead_code)]
struct Wallet {
    balance: f32,
}

pub struct Packet {
    pub security: String,
    pub min_count: i32,
    pub purchased: i32,
    pub profit: f32,
    pub balance: f32,
}

#[allow(dead_code)]
impl Packet {
    pub fn new(security: &str, min_count: i32, balance: f32) -> Self {
        Self {
            security: security.to_string(),
            min_count,
            purchased: 0,
            profit: 0.0,
            balance,
        }
    }
}
