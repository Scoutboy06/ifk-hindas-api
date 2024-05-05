use axum::{extract::Query, http::Method, routing::get, Router};
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

const SCHEDULE_BASE_URL: &'static str =
    "https://www.ifkhindas.com/kalender/ajaxKalender.asp?ID=436811";

fn schedule_url(month: u32, year: u32) -> String {
    let mut result = String::from(SCHEDULE_BASE_URL);

    result.push_str("&manad=");
    result.push_str(&month.to_string());

    result.push_str("&ar=");
    result.push_str(&double_digit(year));

    result
}

fn double_digit(n: u32) -> String {
    let mut result = String::new();

    if n < 10 {
        result.push('0');
    }

    result += &n.to_string();
    result
}

#[derive(Debug, Serialize)]
enum EventCategory {
    Training,
    Competition,
    Other,
}

#[derive(Debug, Serialize)]
struct Event {
    title: String,
    start: String,
    end: String,
    category: EventCategory,
}

trait SelectSingle {
    fn select_single(&self, query: &str) -> Option<ElementRef>;
}

impl SelectSingle for ElementRef<'_> {
    fn select_single(&self, query: &str) -> Option<ElementRef<'_>> {
        self.select(&Selector::parse(query).unwrap()).next()
    }
}

impl SelectSingle for Html {
    fn select_single(&self, query: &str) -> Option<ElementRef> {
        self.select(&Selector::parse(query).unwrap()).next()
    }
}

async fn hello_world() -> &'static str {
    "Hello, world!"
}

#[derive(Deserialize)]
struct CalendarParams {
    month: u32,
    year: u32,
}

async fn calendar(query: Query<CalendarParams>) -> String {
    let response = reqwest::get(schedule_url(query.month, query.year)).await;
    let html = response.unwrap().text().await.unwrap();

    let document = Html::parse_document(&html);

    // (month, year)
    let date = document
        .select_single("body > div.inner > div:nth-child(4) > b")
        .unwrap()
        .text()
        .next()
        .unwrap()
        .split_once(' ')
        .unwrap();

    let month = match date.0 {
        "JANUARI" => "01",
        "FEBRUARI" => "02",
        "MARS" => "03",
        "APRIL" => "04",
        "MAJ" => "05",
        "JUNI" => "06",
        "JULI" => "07",
        "AUGUSTI" => "08",
        "SEPTEMBER" => "09",
        "OKTOBER" => "10",
        "NOVEMBER" => "11",
        "DECEMBER" => "12",
        _ => unreachable!(),
    };

    let year = date.1;

    let mut events: Vec<Event> = Vec::new();

    for day in document.select(&Selector::parse("table.mCal > tbody > tr").unwrap()) {
        if day.select_single("td > table > tbody").is_none() {
            continue;
        }

        let start_date = day
            .select_single("td:nth-child(2) b")
            .unwrap()
            .text()
            .next()
            .unwrap()
            .to_owned();

        let time = day
            .select_single("td > table > tbody > tr > td > div:nth-child(2) > span")
            .unwrap()
            .text()
            .next()
            .unwrap()
            .split_once(" - ")
            .unwrap()
            .to_owned();

        let event_name = day
            .select_single("a.kal")
            .unwrap()
            .text()
            .next()
            .unwrap()
            .to_owned();

        let event_category = match day
            .select_single(".hidden-phone > .calBox")
            .unwrap()
            .attr("class")
            .unwrap()
            .split(" ")
            .nth(2)
            .unwrap()
            .chars()
            .nth(6)
            .unwrap()
        {
            '1' => EventCategory::Training,
            '2' => EventCategory::Competition,
            _ => EventCategory::Other,
        };

        let start = {
            let mut result = String::from(year);
            result.push('-');
            result.push_str(month);
            result.push('-');
            result.push_str(start_date.as_str());
            result.push('T');
            result.push_str(time.0);
            result.push_str(":00");
            result
        };

        let end = {
            let mut result = String::from(year);
            result.push('-');
            result.push_str(month);
            result.push('-');
            result.push_str(start_date.as_str());
            result.push('T');
            result.push_str(time.1);
            result.push_str(":00");
            result
        };

        events.push(Event {
            title: event_name,
            start,
            end,
            category: event_category,
        });
    }

    serde_json::to_string(&events).unwrap()
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    let router = Router::new()
        .route("/", get(hello_world))
        .route("/calendar", get(calendar))
        .layer(cors);

    Ok(router.into())
}
