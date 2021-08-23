use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Duration, FixedOffset, Local, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::influxdb::FluxValue;

use super::{
    chart_data_body, BodyResult, ChartHandler, ClientRequest, CsvDeserializer, FluxParams,
};

type ChartData = Vec<DataSerie>;

#[derive(Clone, Deserialize, Serialize)]
struct DataSerie {
    name: String,
    data: Vec<(i64, u32)>,
}

#[derive(Deserialize)]
struct QueryData {
    // Array of machines indexes strings
    machines: Vec<String>,
    seed: ChartData,
}

#[derive(Deserialize)]
struct ResultRow {
    #[serde(rename(deserialize = "_start"))]
    start: DateTime<FixedOffset>,
    #[serde(rename(deserialize = "_stop"))]
    stop: DateTime<FixedOffset>,
    total: u32,
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
        let start = {
            let now = Local::now();
            let first_shift_end = now.date().and_time(NaiveTime::from_hms(5, 30, 0)).unwrap();
            let shift_duration = Duration::hours(8);
            let start = (0..=3)
                .map(|i| first_shift_end + shift_duration * i)
                .find(|shift_end| now < *shift_end)
                .unwrap();
            start - shift_duration
        };
        let total_function = self
            .query_data
            .machines
            .iter()
            .map(|index| format!("r[\"{}\"]", index))
            .collect::<Vec<_>>()
            .join(" + ");
        let total_function = format!("(r) => {}", total_function);
        flux_params.extend([
            ("machine_set", self.query_data.machines.to_owned().into()),
            ("start", start.into()),
            ("total_function", FluxValue::RawExpression(total_function)),
        ]);
    }

    async fn body(&self, deserializer: CsvDeserializer) -> BodyResult {
        chart_data_body(deserializer, self.query_data.seed.clone(), accumulate).await
    }
}

async fn accumulate(mut acc: ChartData, row: ResultRow) -> tide::Result<ChartData> {
    let serie = acc
        .get_mut(0)
        .ok_or_else(|| anyhow!("missing data serie in seed"))?;
    let start_ms = row.start.timestamp_millis();
    let stop_ms = row.stop.timestamp_millis();
    serie.data.push((start_ms, row.total));
    serie.data.push((stop_ms, row.total));
    Ok(acc)
}
