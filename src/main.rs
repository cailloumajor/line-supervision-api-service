use std::sync::Arc;

use anyhow::Context;

mod chart_data;
mod config;
mod handlers;
mod influxdb;
mod ui_customization;

use config::Config;
use ui_customization::UiCustomization;

#[derive(Clone)]
pub struct AppState {
    influxdb_client: influxdb::Client,
    ui_customization: Arc<UiCustomization>,
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    tide::log::start();

    let config = Config::get()?;
    let logo_file = config.logo_file.clone();
    let ui_customization = UiCustomization::new(&config.ui_customization_file)?;
    let state = AppState {
        influxdb_client: influxdb::Client::new(&config),
        ui_customization: Arc::new(ui_customization),
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
