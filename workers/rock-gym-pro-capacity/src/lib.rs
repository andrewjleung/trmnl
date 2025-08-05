use scraper::Selector;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use worker::*;

const GYM_ID: &str = "dd60512aa081d8b38fff4ddbbd364a54";
const ROCK_GYM_PRO_BASE_URL: &str = "https://portal.rockgympro.com";

#[derive(Deserialize, Serialize, Clone)]
struct GymCapacity {
    id: String,
    name: String,
    capacity: u32,
    count: u32,
}

#[event(start)]
fn start() {
    console_error_panic_hook::set_once();
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let authorization_header = match req.headers().get("Authorization") {
        Ok(Some(value)) => value,
        _ => return Response::error("Unauthorized", 401),
    };

    let token = match authorization_header.strip_prefix("Bearer ") {
        Some(token) => token,
        None => return Response::error("Unauthorized", 401),
    };

    if token != env.secret("ACCESS_TOKEN")?.to_string() {
        return Response::error("Unauthorized", 401);
    }

    let content_type = match get_content_type(&req) {
        Some(content_type) => content_type,
        None => return Response::error("Missing content-type header", 400),
    };

    if !content_type.contains("application/json") {
        return Response::error("Invalid content type", 415);
    }

    if !matches!(req.method(), Method::Get) {
        return Response::error("Method Not Allowed", 405);
    }

    let html_text = match get_html_text().await {
        Some(html_data) => html_data,
        None => return Response::error("Failed to fetch RockGymPro data", 500),
    };

    let rgp_data_unparsed = match parse_html_capacities(&html_text) {
        Some(rgp_data_unparsed) => rgp_data_unparsed,
        None => return Response::error("Failed to read RockGymPro data", 500),
    };

    let facility_names = parse_html_facility_names(&html_text);

    let rgp_data = match deserialize_rgp_data(&rgp_data_unparsed) {
        Some(rgp_data) => rgp_data,
        None => return Response::error("Failed to parse RockGymPro data", 500),
    };

    let rgp_data = HashMap::from([(
        "gyms",
        rgp_data
            .iter()
            .map(|(gym, data)| GymCapacity {
                id: gym.to_owned(),
                name: facility_names.get(gym).unwrap_or(gym).to_owned(),
                count: data.count,
                capacity: data.capacity,
            })
            .collect::<Vec<GymCapacity>>(),
    )]);

    match json5::to_string(&rgp_data) {
        Ok(json_str) => Response::ok(json_str),
        Err(e) => Response::error(format!("Failed to serialise data: {e}"), 500),
    }
}

fn get_content_type(req: &Request) -> Option<String> {
    let content_type = match req.headers().get("content-type") {
        Ok(content_type) => match content_type {
            Some(content_type) => content_type,
            None => "".to_string(),
        },
        Err(_) => return None,
    };

    Some(content_type)
}

async fn get_html_text() -> Option<String> {
    let html_url = format!("{ROCK_GYM_PRO_BASE_URL}/portal/public/{GYM_ID}/occupancy");

    let html_text = match reqwest::get(html_url).await {
        Ok(body) => match body.text().await {
            Ok(html_text) => html_text,
            Err(_) => return None,
        },
        Err(_) => return None,
    };

    Some(html_text)
}

fn parse_html_capacities(html_text: &str) -> Option<String> {
    let document = scraper::Html::parse_document(html_text);
    let script_selector = Selector::parse("script").unwrap();
    let capacities_re = regex::Regex::new(r"(?s)var\s+data\s*?=\s*?(\{.*?\})\;").unwrap();

    for script in document.select(&script_selector) {
        let script_text = script.text().collect::<Vec<_>>().join("");

        if !script_text.contains("var data =") {
            continue;
        }

        // Group[0] is the full string
        if let Some(match_) = capacities_re.captures(&script_text) {
            return Some(match_[1].to_string());
        };
    }

    None
}

fn parse_html_facility_names(html_text: &str) -> HashMap<String, String> {
    let document = scraper::Html::parse_document(html_text);
    let option_selector = Selector::parse("option").unwrap();

    document
        .select(&option_selector)
        .filter_map(|option| {
            if let Some(value) = option.value().attr("value") {
                if value.is_empty() {
                    return None;
                }

                return Some((value.to_owned(), option.inner_html()));
            }

            None
        })
        .collect::<HashMap<String, String>>()
}

#[derive(Deserialize, Serialize, Clone)]
struct RGPData {
    capacity: u32,
    count: u32,
}

fn deserialize_rgp_data(rgp_data_unparsed: &str) -> Option<HashMap<String, RGPData>> {
    // The HTML response contains JavaScript, not real JSON
    // json5 is more lenient than serde which only accepts strict compliance
    json5::from_str(rgp_data_unparsed).ok()
}
