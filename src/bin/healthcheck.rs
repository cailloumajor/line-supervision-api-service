use std::env;

use anyhow::{anyhow, Context};

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let port_arg = args
        .get(1)
        .context("Missing port number as first argument")?;
    let port: u16 = port_arg
        .parse()
        .with_context(|| format!(r#"Invalid port number from first argument: "{}""#, port_arg))?;

    let status = surf::get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .map_err(|e| e.into_inner())?
        .status();
    status
        .is_success()
        .then(|| ())
        .ok_or_else(|| anyhow!("Bad status code: {}", status))
}
