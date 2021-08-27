use std::str::FromStr;

use anyhow::anyhow;
use async_trait::async_trait;
use csv_async::AsyncDeserializer;
use futures::{StreamExt, TryFuture, TryStreamExt};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::influxdb::FluxParams;
use crate::AppState;

mod machine_state;
mod production;

type BodyResult = tide::Result<tide::Body>;
type ClientRequest = tide::Request<AppState>;
type CsvDeserializer = AsyncDeserializer<surf::Response>;

#[async_trait]
trait ChartHandler {
    fn set_params(&self, flux_params: &mut FluxParams);
    async fn body(&self, deserializer: CsvDeserializer) -> BodyResult;
}

#[derive(Debug)]
enum Chart {
    MachinesState,
    Production,
}

impl Chart {
    async fn handler(
        &self,
        req: &mut ClientRequest,
    ) -> tide::Result<Box<dyn ChartHandler + Send + Sync>> {
        match self {
            Self::MachinesState => Ok(Box::new(
                self::machine_state::Handler::from_request(req).await?,
            )),
            Self::Production => Ok(Box::new(
                self::production::Handler::from_request(req).await?,
            )),
        }
    }

    fn template(&self) -> &'static str {
        match self {
            Self::MachinesState => include_str!("machines_state.flux"),
            Self::Production => include_str!("production.flux"),
        }
    }
}

impl FromStr for Chart {
    type Err = tide::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "machines-state" => Ok(Self::MachinesState),
            "production" => Ok(Self::Production),
            _ => Err(tide::Error::new(
                404,
                anyhow!("unknown route parameter: {}", s),
            )),
        }
    }
}

async fn chart_data_body<T, F, Fut, R>(
    mut deserializer: CsvDeserializer,
    seed: T,
    accumulate: F,
) -> BodyResult
where
    F: FnMut(T, R) -> Fut,
    Fut: TryFuture<Ok = T, Error = tide::Error>,
    R: DeserializeOwned,
    T: Serialize,
{
    let chart_data = deserializer
        .deserialize()
        .map(|r| r.map_err(tide::Error::from))
        .try_fold(seed, accumulate)
        .await?;
    Ok(tide::Body::from_json(&chart_data)?)
}

pub async fn handler(mut req: ClientRequest) -> tide::Result {
    let chart: Chart = req.param("name").unwrap().parse()?;
    let chart_handler = chart.handler(&mut req).await?;
    let mut flux_params = FluxParams::new();
    chart_handler.set_params(&mut flux_params);
    let influxdb_res = req
        .state()
        .influxdb_client
        .flux_query(chart.template(), flux_params)
        .await?;
    let deserializer = AsyncDeserializer::from_reader(influxdb_res);
    let body = chart_handler.body(deserializer).await?;
    Ok(body.into())
}
