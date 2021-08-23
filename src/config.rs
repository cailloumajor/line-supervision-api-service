use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{Context, Result};
use dotenv::dotenv;
use envconfig::Envconfig;
use url::Url;

#[derive(Clone)]
pub struct BaseUrl(Url);

impl FromStr for BaseUrl {
    type Err = url::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut base_path = s.to_string();
        if !base_path.ends_with('/') {
            base_path.push('/');
        }
        let base_url: Url = base_path.parse()?;

        Ok(Self(base_url))
    }
}

impl From<BaseUrl> for Url {
    fn from(base: BaseUrl) -> Self {
        base.0
    }
}

#[derive(Envconfig)]
pub struct Config {
    pub ui_customization_file: PathBuf,
    pub logo_file: PathBuf,
    pub influxdb_base_url: BaseUrl,
    pub influxdb_read_token: String,
    pub influxdb_org: String,
    pub influxdb_bucket: String,
}

impl Config {
    pub fn get() -> Result<Self> {
        dotenv().context("failed to get environment variables from .env file")?;
        let config = Config::init_from_env()
            .context("failed to get configuration from environment variables")?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;

    #[test_case("http://:8000" => panics "EmptyHost" ; "URL parse error")]
    #[test_case("http://host:8080" => "http://host:8080/" ; "appends slash")]
    #[test_case("http://host:8000/" => "http://host:8000/" ; "keeps slash")]
    #[test_case("http://host/path" => "http://host/path/" ; "appends slash to path")]
    #[test_case("http://host/path/" => "http://host/path/" ; "keeps slash after path")]
    fn base_url_from_str(string: &str) -> String {
        let base_url: BaseUrl = string.parse().unwrap();

        base_url.0.into()
    }

    #[test]
    fn url_from_base_url() {
        let url = Url::parse("http://host:80").unwrap();
        let base_url = BaseUrl(url.clone());
        let into_url: Url = base_url.into();
        assert_eq!(into_url, url)
    }
}
