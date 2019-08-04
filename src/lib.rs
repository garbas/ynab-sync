use chrono::NaiveDate;
use serde::de::{self, Deserializer, Visitor};
use std::fmt;
use std::result;

pub mod error;
// TODO: pub mod rules;
pub mod ingdiba;
pub mod n26;
pub mod ynab;

pub use error::{Error, ErrorKind, Result};
pub use ingdiba::IngDiBa;
pub use n26::N26;
pub use ynab::YNAB;

fn convert_to_int<'de, D>(deserializer: D) -> result::Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    struct I32Visitor;

    impl<'de> Visitor<'de> for I32Visitor {
        type Value = i32;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a cent representation in i32 of an amount provided in f64")
        }
        fn visit_f64<E>(self, value: f64) -> result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(((value * 1000.0).round()) as Self::Value)
        }
    }

    deserializer.deserialize_f64(I32Visitor)
}

fn convert_to_int_eu_style<'de, D>(deserializer: D) -> result::Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    struct I32Visitor;

    impl<'de> Visitor<'de> for I32Visitor {
        type Value = i32;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter
                .write_str("a cent representation in i32 of an amount provided in f64 in eu style")
        }
        fn visit_str<E>(self, s: &str) -> result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            let float = s.replace(".", "").replace(",", ".");
            match float.parse::<f64>() {
                Ok(x) => Ok(((x * 1000.0).round()) as Self::Value),
                Err(e) => Err(E::custom(format!("Parse error {} for {}", e, float))),
            }
        }
    }

    deserializer.deserialize_str(I32Visitor)
}

fn convert_to_local_date<'de, D>(deserializer: D) -> result::Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    struct StrVisitor;

    impl<'de> Visitor<'de> for StrVisitor {
        type Value = NaiveDate;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a local date representation in YYYY-MM-DD format")
        }
        fn visit_str<E>(self, s: &str) -> result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            NaiveDate::parse_from_str(s, "%d.%m.%Y")
                .map_err(|e| E::custom(format!("Parse error {} for {}", e, s)))
        }
    }

    deserializer.deserialize_str(StrVisitor)
}
