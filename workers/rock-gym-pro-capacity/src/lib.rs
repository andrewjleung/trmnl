use scraper::error::SelectorErrorKind;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::result::Result::{self as StdResult, Err as StdErr, Ok as StdOk};
use worker::*;

const GYM_ID: &str = "dd60512aa081d8b38fff4ddbbd364a54";
const ROCK_GYM_PRO_BASE_URL: &str = "https://portal.rockgympro.com";

#[event(start)]
fn start() {
    console_error_panic_hook::set_once();
}

#[event(fetch)]
async fn fetch(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    if !matches!(req.method(), Method::Get) {
        return Response::error("Method Not Allowed", 405);
    }

    Response::error("Bad Request", 400)
}

struct Occupancy {
    count: u16,
    capacity: u16,
}

enum WorkerError {
    RequestError(reqwest::Error),
}

impl From<reqwest::Error> for WorkerError {
    fn from(error: reqwest::Error) -> Self {
        WorkerError::RequestError(error)
    }
}

async fn fetch_occupancies() -> StdResult<HashMap<String, Occupancy>, WorkerError> {
    let url = format!("{ROCK_GYM_PRO_BASE_URL}/portal/public/{GYM_ID}/occupancy");

    let response = reqwest::Client::new()
        .get(url)
        .header("Accept", "text/html")
        .send()
        .await?
        .text()
        .await?;

    let document = Html::parse_document(&response);
    // let script_selector = Selector::parse("script")?;

    Ok(HashMap::new())
}

