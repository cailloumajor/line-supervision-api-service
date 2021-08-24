use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::fs;
use toml::Value as TomlValue;

use crate::config::Config;

fn camel_case(source: &str) -> String {
    let mut dest = String::with_capacity(source.len());
    let mut capitalize = false;
    for ch in source.chars() {
        if ch == ' ' || ch == '_' {
            capitalize = true;
        } else if capitalize {
            dest.push(ch.to_ascii_uppercase());
            capitalize = false;
        } else {
            dest.push(ch);
        }
    }
    dest[..1].to_ascii_lowercase() + &dest[1..]
}

fn convert(toml_value: TomlValue) -> JsonValue {
    match toml_value {
        TomlValue::Array(arr) => JsonValue::Array(arr.into_iter().map(convert).collect()),
        TomlValue::Boolean(b) => JsonValue::Bool(b),
        TomlValue::Datetime(dt) => JsonValue::String(dt.to_string()),
        TomlValue::Float(f) => match serde_json::Number::from_f64(f) {
            Some(n) => JsonValue::Number(n),
            None => JsonValue::Null,
        },
        TomlValue::Integer(i) => JsonValue::Number(i.into()),
        TomlValue::String(s) => JsonValue::String(s),
        TomlValue::Table(table) => JsonValue::Object(
            table
                .into_iter()
                .map(|(k, v)| (camel_case(&k), convert(v)))
                .collect(),
        ),
    }
}

fn toml_to_json(source: &str) -> Result<String> {
    let toml = source.parse().context("failed to parse TOML")?;
    let json = serde_json::to_string(&convert(toml)).context("failed serializing JSON")?;
    Ok(json)
}

pub fn get_ui_customization(config: &Config) -> Result<String> {
    let toml_file = &config.ui_customization_file;
    let raw_toml = fs::read_to_string(toml_file)
        .with_context(|| format!("failed reading {}", toml_file.display()))?;
    let ui_customization = toml_to_json(&raw_toml)
        .with_context(|| format!("failed to convert {} to JSON", toml_file.display()))?;
    Ok(ui_customization)
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::Path;

    use test_case::test_case;

    use super::*;

    #[test_case("oneword" => "oneword")]
    #[test_case("camelCase" => "camelCase")]
    #[test_case("space separated" => "spaceSeparated")]
    #[test_case("underscore_separated" => "underscoreSeparated")]
    #[test_case("multiple   spaces" => "multipleSpaces")]
    #[test_case("   spaces_underscores_mixed" => "spacesUnderscoresMixed")]
    fn camel_case(src: &str) -> String {
        super::camel_case(src)
    }

    #[test]
    fn convert_snapshot() {
        let toml_file = Path::new(file!()).parent().unwrap().join("test.toml");
        let toml_data = fs::read_to_string(toml_file).unwrap();
        insta::assert_yaml_snapshot!(super::convert(toml_data.parse().unwrap()));
    }
}
