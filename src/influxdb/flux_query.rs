use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::{DateTime, SecondsFormat, TimeZone, Utc};
use regex::Regex;

pub type FluxParams = HashMap<&'static str, FluxValue>;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Comparable {
    String(String),
    Time(DateTime<Utc>),
}

impl Comparable {
    fn to_flux_repr(&self) -> String {
        match self {
            Self::String(s) => format!("\"{}\"", s),
            Self::Time(t) => t.to_rfc3339_opts(SecondsFormat::AutoSi, true),
        }
    }
}

impl From<String> for Comparable {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl<Tz> From<DateTime<Tz>> for Comparable
where
    Tz: TimeZone,
    DateTime<Tz>: Into<DateTime<Utc>>,
{
    fn from(value: DateTime<Tz>) -> Self {
        Self::Time(value.into())
    }
}

#[derive(Debug)]
pub enum FluxValue {
    Comparable(Comparable),
}

impl FluxValue {
    fn to_flux_repr(&self) -> String {
        match self {
            Self::Comparable(comp) => comp.to_flux_repr(),
        }
    }
}

impl<V> From<V> for FluxValue
where
    V: Into<Comparable>,
{
    fn from(value: V) -> Self {
        Self::Comparable(value.into())
    }
}

pub(crate) struct QueryBuilder {
    re: Regex,
}

impl QueryBuilder {
    pub(super) fn new() -> Self {
        let re = Regex::new(r"__(\w+)__").unwrap();
        QueryBuilder { re }
    }

    pub(super) fn generate_query(&self, template: &str, params: FluxParams) -> Result<String> {
        // filter out lines defining placeholders variables
        let source = template
            .lines()
            .filter(|l| self.re.find(l.trim_start()).map(|m| m.start()) != Some(0))
            .fold(String::new(), |s, l| s + l + "\n");

        let mut query = String::with_capacity(source.len());
        let mut last_match = 0;
        for caps in self.re.captures_iter(&source) {
            let full_match = caps.get(0).unwrap();
            let param_key = caps.get(1).unwrap().as_str();
            let replacement = params
                .get(param_key)
                .with_context(|| format!("missing `{}` parameter", param_key))?;
            query.push_str(&source[last_match..full_match.start()]);
            query.push_str(&replacement.to_flux_repr());
            last_match = full_match.end();
        }
        query.push_str(&source[last_match..]);

        Ok(query)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use assert_matches::assert_matches;
    use chrono::Local;
    use indoc::indoc;
    use test_case::test_case;

    use super::*;

    #[test_case(Comparable::String("string".to_string()) => r#""string""# ; "string")]
    #[test_case(
        Comparable::Time(Utc.ymd(2014, 7, 8).and_hms(9, 10, 11))
        => "2014-07-08T09:10:11Z"
        ; "time"
    )]
    fn comparable_to_flux_repr(comparable: Comparable) -> String {
        comparable.to_flux_repr()
    }

    #[test]
    fn flux_value_from_string() {
        let flux_value: FluxValue = "string".to_string().into();
        assert_matches!(flux_value, FluxValue::Comparable(Comparable::String(s)) => {
            assert_eq!(s, "string")
        })
    }

    #[test]
    fn flux_value_from_local_time() {
        let local_time = Local.ymd(2020, 5, 14).and_hms(17, 23, 18);
        let flux_value: FluxValue = local_time.into();
        assert_matches!(flux_value, FluxValue::Comparable(Comparable::Time(t)) => {
            assert_eq!(t, local_time.with_timezone(&Utc))
        })
    }

    #[test]
    fn flux_value_comparable_to_flux_repr() {
        let comparable = Comparable::String("string".to_string());
        let comp_repr = comparable.to_flux_repr();
        let value = FluxValue::Comparable(comparable);
        assert_eq!(value.to_flux_repr(), comp_repr)
    }

    const TEMPLATE: &str = indoc! {r#"
        __var__ = "value1"
        Line using placeholder: __var__
    "#};

    #[test]
    fn generate_with_missing_parameter() {
        let builder = QueryBuilder::new();
        let mut params = HashMap::new();
        params.insert("unknown_var", "Value_1".to_string().into());
        assert!(builder.generate_query(TEMPLATE, params).is_err());
    }

    #[test]
    fn generate_success() {
        let builder = QueryBuilder::new();
        let expected = String::from(indoc! {r#"
            Line using placeholder: "string_value"
        "#});
        let mut params = HashMap::new();
        params.insert("var", "string_value".to_string().into());
        assert_eq!(builder.generate_query(TEMPLATE, params).unwrap(), expected);
    }
}
