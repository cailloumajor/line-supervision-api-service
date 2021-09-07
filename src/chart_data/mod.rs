use std::str::FromStr;

use anyhow::anyhow;
use async_trait::async_trait;
use csv_async::AsyncDeserializer;
use futures::{StreamExt, TryStreamExt};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tide::StatusCode;

use crate::influxdb::{FluxParams, FluxValue};
use crate::AppState;

pub mod machine_state;
pub mod production;

type AddFluxParams = Vec<(&'static str, FluxValue)>;
type ClientRequest = tide::Request<AppState>;

#[async_trait]
trait ChartHandler {
    type ChartData;
    type ResultRow: DeserializeOwned;

    fn time_bounds(&self) -> (String, String);
    fn flux_template(&self) -> &str;
    fn flux_params(&self) -> AddFluxParams;
    async fn accumulate(
        &self,
        acc: Self::ChartData,
        row: Self::ResultRow,
    ) -> tide::Result<Self::ChartData>;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommonQueryData<T> {
    seed: T,
}

enum Chart {
    MachinesState,
    Production,
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

async fn handle<H, T>(mut req: ClientRequest, chart_handler: &H) -> tide::Result
where
    H: ChartHandler<ChartData = T>,
    T: DeserializeOwned + Serialize,
{
    let req_body = req.body_string().await?;
    let mut response = tide::Response::new(StatusCode::Ok);
    let time_bounds = chart_handler.time_bounds();
    response.insert_header("Chart-Start-Time", time_bounds.0);
    response.insert_header("Chart-End-Time", time_bounds.1);
    let mut flux_params = FluxParams::new();
    flux_params.extend(chart_handler.flux_params());
    let query_data: CommonQueryData<T> = serde_json::from_str(&req_body)?;
    let influxdb_res = req
        .state()
        .influxdb_client
        .flux_query(chart_handler.flux_template(), flux_params)
        .await?;
    let chart_data = AsyncDeserializer::from_reader(influxdb_res)
        .deserialize()
        .map(|r| r.map_err(tide::Error::from))
        .try_fold(query_data.seed, |acc, row| async move {
            chart_handler.accumulate(acc, row).await
        })
        .await?;
    response.set_body(tide::Body::from_json(&chart_data)?);
    Ok(response)
}
