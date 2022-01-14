use std::sync::Arc;

use anyhow::Context;
use async_std::sync::Mutex;
use async_std::task;
use futures::future::{AbortHandle, Abortable, Aborted};
use futures::StreamExt;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::low_level::signal_name;
use signal_hook_async_std::Signals;
use tide::log;

mod chart_data;
mod config;
mod handlers;
mod influxdb;
mod ui_customization;

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

async fn handle_signals(mut signals: Signals, abort_handle: AbortHandle) {
    while let Some(signal) = signals.next().await {
        let signame = signal_name(signal).unwrap_or("unknown");
        log::info!("Received {} signal, stopping", signame);
        abort_handle.abort();
    }
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    log::start();

    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let signals = Signals::new(TERM_SIGNALS)?;
    let signals_handle = signals.handle();
    let signals_task = task::spawn(handle_signals(signals, abort_handle));

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
        .with_context(|| format!("logo file path error: {}", logo_file.display()))?;
    app.at("/ui-customization").get(handlers::ui_customization);
    app.at("/influxdb-ready").get(handlers::influxdb_ready);
    app.at("/chart-data/machines-state")
        .post(chart_data::machine_state::handler);
    app.at("/chart-data/production")
        .post(chart_data::production::handler);

    let listen_future = Abortable::new(app.listen("0.0.0.0:8080"), abort_registration);
    match listen_future.await {
        Err(Aborted) => Ok(()),
        Ok(listen_result) => listen_result,
    }?;

    signals_handle.close();
    signals_task.await;

    Ok(())
}
