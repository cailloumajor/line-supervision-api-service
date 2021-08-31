use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Duration, FixedOffset, Local, NaiveTime};
use serde::{Deserialize, Serialize};

use super::{handle, AddFluxParams, ChartHandler, ClientRequest};

type ChartData = Vec<DataSerie>;

#[derive(Clone, Deserialize, Serialize)]
struct DataSerie {
    name: String,
    data: Vec<(i64, u32)>,
}

#[derive(Deserialize)]
struct ResultRow {
    #[serde(rename(deserialize = "_start"))]
    start: DateTime<FixedOffset>,
    #[serde(rename(deserialize = "_stop"))]
    stop: DateTime<FixedOffset>,
    total: u32,
}

struct Handler {
    start_time: DateTime<Local>,
    end_time: DateTime<Local>,
}

impl Handler {
    pub fn new() -> Self {
        let now = Local::now();
        let first_shift_end = now.date().and_time(NaiveTime::from_hms(5, 30, 0)).unwrap();
        let shift_duration = Duration::hours(8);
        let end_time = (0..=3)
            .map(|i| first_shift_end + shift_duration * i)
            .find(|&shift_end| now < shift_end)
            .unwrap();
        let start_time = end_time - shift_duration;
        Self {
            start_time,
            end_time,
        }
    }
}

#[async_trait]
impl ChartHandler for Handler {
    type ChartData = ChartData;
    type ResultRow = ResultRow;

    fn time_bounds(&self) -> (String, String) {
        (
            self.start_time.timestamp_millis().to_string(),
            self.end_time.timestamp_millis().to_string(),
        )
    }

    fn flux_params(&self) -> AddFluxParams {
        vec![("start", self.start_time.into())]
    }

    async fn accumulate(&self, mut acc: ChartData, row: ResultRow) -> tide::Result<ChartData> {
        let serie = acc
            .get_mut(0)
            .ok_or_else(|| anyhow!("missing data serie in seed"))?;
        let start_ms = row.start.timestamp_millis();
        let stop_ms = row.stop.timestamp_millis();
        serie.data.push((start_ms, row.total));
        serie.data.push((stop_ms, row.total));
        Ok(acc)
    }
}

pub async fn handler(req: ClientRequest) -> tide::Result {
    let chart_handler = Handler::new();
    handle(req, &chart_handler).await
}
