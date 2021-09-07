use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct UiCustomizationData {
    html_title: String,
    app_title: String,
    synoptics: Synoptics,
    pub machines: Vec<Machine>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct Synoptics {
    viewbox: Dimensions,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct Dimensions {
    width: u32,
    height: u32,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct Machine {
    pub name: String,
    path: String,
    tag_pos: Coordinates,
    card_pos: Coordinates,
    #[serde(default)]
    campaign: bool,
    #[serde(default)]
    pub production: bool,
    #[serde(default)]
    pub state_chart: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
struct Coordinates {
    x: i32,
    y: i32,
}

pub struct UiCustomization {
    pub config: UiCustomizationData,
    pub json: String,
}

impl UiCustomization {
    pub fn new(toml_file: &Path) -> Result<Self> {
        let raw_toml = fs::read_to_string(toml_file)
            .with_context(|| format!("failed reading {}", toml_file.display()))?;
        let config = toml::from_str(&raw_toml)
            .with_context(|| format!("failed to deserialize {}", toml_file.display()))?;
        let json = serde_json::to_string(&config)
            .context("failed to serialize UI customization data to JSON")?;
        Ok(Self { config, json })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn ui_customization_snapshot() {
        let toml_file = Path::new(file!())
            .parent()
            .unwrap()
            .join("../ui_data/customization.toml");
        let ui_customization = UiCustomization::new(&toml_file).unwrap();
        insta::assert_yaml_snapshot!(ui_customization.config);
        insta::assert_snapshot!(ui_customization.json);
    }
}
