use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{
    chart_data_body, BodyResult, ChartHandler, ClientRequest, CsvDeserializer, FluxParams,
};

type ChartData = Vec<DataSerie>;

#[derive(Clone, Deserialize, Serialize)]
struct DataSerie {
    name: String,
    data: Vec<DataPoint>,
}

#[derive(Clone, Deserialize, Serialize)]
struct DataPoint {
    x: String,
    y: (i64, i64),
}

#[derive(Deserialize)]
struct QueryData {
    // Map of machines indexes to their name
    #[serde(with = "indexmap::serde_seq")]
    machines: IndexMap<String, String>,
    seed: ChartData,
}

#[derive(Deserialize)]
struct ResultRow {
    #[serde(rename(deserialize = "_time"))]
    time: DateTime<FixedOffset>,
    duration: i64,
    machine_name: String,
    state_index: usize,
}

pub struct Handler {
    query_data: QueryData,
}

impl Handler {
    pub async fn from_request(request: &mut ClientRequest) -> tide::Result<Self> {
        let query_data = request.body_json().await?;
        Ok(Self { query_data })
    }
}

#[async_trait]
impl ChartHandler for Handler {
    fn set_params(&self, flux_params: &mut FluxParams) {
        let machine_set = self.query_data.machines.keys().cloned().collect::<Vec<_>>();
        flux_params.extend([
            ("machine_set", machine_set.into()),
            ("machines", self.query_data.machines.to_owned().into()),
        ]);
    }

    async fn body(&self, deserializer: CsvDeserializer) -> BodyResult {
        chart_data_body(deserializer, self.query_data.seed.clone(), accumulate).await
    }
}

async fn accumulate(mut acc: ChartData, row: ResultRow) -> tide::Result<ChartData> {
    let state = acc
        .get_mut(row.state_index)
        .ok_or_else(|| anyhow!("seed is missing state with index {}", row.state_index))?;
    let start_time = row.time.timestamp_millis();
    let end_time = start_time + row.duration * 1000;
    state.data.push(DataPoint {
        x: row.machine_name,
        y: (start_time, end_time),
    });
    Ok(acc)
}
