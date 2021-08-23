use tide::http::mime;
use tide::{Request, Response, StatusCode};

use crate::AppState;

pub async fn health(_req: Request<AppState>) -> tide::Result {
    Ok(StatusCode::NoContent.into())
}

pub async fn ui_customization(req: Request<AppState>) -> tide::Result {
    let body = req.state().ui_customization_json.as_str();
    Ok(Response::builder(StatusCode::Ok)
        .content_type(mime::JSON)
        .body(body)
        .build())
}

pub async fn influxdb_ready(req: Request<AppState>) -> tide::Result {
    Ok(Response::from_res(
        req.state().influxdb_client.ready().await?,
    ))
}
