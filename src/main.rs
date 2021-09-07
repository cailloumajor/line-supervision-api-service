use std::sync::Arc;

use anyhow::Context;

mod chart_data;
mod config;
mod handlers;
mod influxdb;
mod ui_customization;

use async_std::sync::Mutex;
use chart_data::machine_state::Handler as MachineStateChart;
use chart_data::production::Handler as ProductionChart;
use config::Config;
use ui_customization::UiCustomization;

#[derive(Clone)]
pub struct AppState {
    influxdb_client: influxdb::Client,
    ui_customization: Arc<UiCustomization>,
    machine_state_chart: Arc<MachineStateChart>,
    production_chart: Arc<Mutex<ProductionChart>>,
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    tide::log::start();

    let config = Config::get()?;
    let logo_file = config.logo_file.clone();
    let ui_customization = UiCustomization::new(&config.ui_customization_file)?;
    let machine_state_chart = MachineStateChart::new(&ui_customization.config);
    let production_chart = ProductionChart::new(&ui_customization.config);

    let state = AppState {
        influxdb_client: influxdb::Client::new(&config),
        ui_customization: ui_customization.into(),
        machine_state_chart: machine_state_chart.into(),
        production_chart: Mutex::new(production_chart).into(),
    };

    let mut app = tide::with_state(state);
    app.at("/health").get(handlers::health);
    app.at("/logo")
        .serve_file(&logo_file)
        .with_context(|| format!("failed to serve {}", logo_file.display()))?;
    app.at("/ui-customization").get(handlers::ui_customization);
    app.at("/influxdb-ready").get(handlers::influxdb_ready);
    app.at("/chart-data/machines-state")
        .post(chart_data::machine_state::handler);
    app.at("/chart-data/production")
        .post(chart_data::production::handler);

    app.listen("0.0.0.0:8080").await?;
    Ok(())
}
