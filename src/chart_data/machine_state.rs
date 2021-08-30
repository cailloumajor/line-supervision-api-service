use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Duration, FixedOffset, Utc};
use serde::{Deserialize, Serialize};

use super::{handle, AddFluxParams, ChartHandler, ClientRequest};

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
struct ResultRow {
    #[serde(rename(deserialize = "_time"))]
    time: DateTime<FixedOffset>,
    duration: i64,
    machine_name: String,
    state_index: usize,
}

struct Handler;

#[async_trait]
impl ChartHandler for Handler {
    type ChartData = ChartData;
    type ResultRow = ResultRow;

    fn time_bounds(&self) -> (String, String) {
        let now = Utc::now();
        let start = now - Duration::hours(24);
        (
            start.timestamp_millis().to_string(),
            now.timestamp_millis().to_string(),
        )
    }

    fn flux_params(&self) -> AddFluxParams {
        Vec::new()
    }

    async fn accumulate(&self, mut acc: ChartData, row: ResultRow) -> tide::Result<ChartData> {
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
}

pub async fn handler(req: ClientRequest) -> tide::Result {
    let chart_handler = Handler {};
    handle(req, &chart_handler).await
}
