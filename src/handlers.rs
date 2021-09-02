use tide::http::mime;
use tide::{Request, Response, StatusCode};

use crate::AppState;

pub async fn health(_req: Request<AppState>) -> tide::Result {
    Ok(StatusCode::NoContent.into())
}

pub async fn ui_customization(req: Request<AppState>) -> tide::Result {
    Ok(Response::builder(StatusCode::Ok)
        .content_type(mime::JSON)
        .body(req.state().ui_customization.json.as_str())
        .build())
}

pub async fn influxdb_ready(req: Request<AppState>) -> tide::Result {
    Ok(Response::from_res(
        req.state().influxdb_client.ready().await?,
    ))
}
