use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{DateTime, Duration, FixedOffset, Local, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};

use super::{handle, AddFluxParams, ChartHandler, ClientRequest};
use crate::influxdb::FluxValue::RawExpression;
use crate::ui_customization::UiCustomizationData;

type ChartData = Vec<DataSerie>;

#[derive(Clone, Deserialize, Serialize)]
pub struct DataSerie {
    name: String,
    data: Vec<(i64, u32)>,
}

#[derive(Deserialize)]
pub struct ResultRow {
    #[serde(rename(deserialize = "_start"))]
    start: DateTime<FixedOffset>,
    #[serde(rename(deserialize = "_stop"))]
    stop: DateTime<FixedOffset>,
    total: u32,
}

pub struct Handler {
    start_time: DateTime<Local>,
    end_time: DateTime<Local>,
    machine_set: Vec<String>,
    total_function: String,
}

impl Handler {
    pub fn new(ui_customization: &UiCustomizationData) -> Self {
        let epoch = Local.timestamp(0, 0);
        let machine_set: Vec<_> = ui_customization
            .machines
            .iter()
            .enumerate()
            .filter_map(|(index, machine)| machine.production.then(|| index.to_string()))
            .collect();
        let total_function = format!(
            "(r) => {}",
            machine_set
                .iter()
                .map(|index| format!("r[\"{}\"]", index))
                .collect::<Vec<_>>()
                .join(" + ")
        );
        Self {
            start_time: epoch,
            end_time: epoch,
            machine_set,
            total_function,
        }
    }

    fn update_time(&mut self) {
        let now = Local::now();
        let first_shift_end = now.date().and_time(NaiveTime::from_hms(5, 30, 0)).unwrap();
        let shift_duration = Duration::hours(8);
        let end_time = (0..=3)
            .map(|i| first_shift_end + shift_duration * i)
            .find(|&shift_end| now < shift_end)
            .unwrap();
        let start_time = end_time - shift_duration;
        self.start_time = start_time;
        self.end_time = end_time;
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

    fn flux_template(&self) -> &str {
        include_str!("production.flux")
    }

    fn flux_params(&self) -> AddFluxParams {
        vec![
            ("start", self.start_time.into()),
            ("machine_set", self.machine_set.clone().into()),
            ("total_function", RawExpression(self.total_function.clone())),
        ]
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
    let mut chart_handler = req.state().production_chart.lock_arc().await;
    chart_handler.update_time();
    handle(req, &*chart_handler).await
}
