use axum::{http::Method, routing::get, Router};
use scraper::{ElementRef, Html, Selector};
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};

const SCHEDULE_URL: &'static str = "https://www.ifkhindas.com/kalender/ajaxKalender.asp?ID=436811";

#[derive(Debug, Serialize)]
enum EventCategory {
    Training,
    Competition,
    Other,
}

#[derive(Debug, Serialize)]
struct Event<'a> {
    date: &'a str,
    start_time: &'a str,
    end_time: &'a str,
    name: &'a str,
    category: EventCategory,
}

trait SelectSingle<'a> {
    fn select_single(&self, query: &str) -> Option<ElementRef<'a>>;
}

impl<'a> SelectSingle<'a> for ElementRef<'a> {
    fn select_single(&self, query: &str) -> Option<ElementRef<'a>> {
        self.select(&Selector::parse(query).unwrap()).next()
    }
}

async fn hello_world() -> &'static str {
    "Hello, world!"
}

async fn calendar() -> String {
    let response = reqwest::get(SCHEDULE_URL).await;
    let html = response.unwrap().text().await.unwrap();

    let document = Html::parse_document(&html);

    let mut events: Vec<Event> = Vec::new();

    for day in document.select(&Selector::parse("table.mCal > tbody > tr").unwrap()) {
        if day.select_single("td > table > tbody").is_none() {
            continue;
        }

        let date = day
            .select_single("td:nth-child(2) b")
            .unwrap()
            .text()
            .next()
            .unwrap();

        let time = day
            .select_single("td > table > tbody > tr > td > div:nth-child(2) > span")
            .unwrap()
            .text()
            .next()
            .unwrap()
            .split_once(" - ")
            .unwrap();

        let event_name = day.select_single("a.kal").unwrap().text().next().unwrap();

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

        events.push(Event {
            date,
            start_time: time.0,
            end_time: time.1,
            name: event_name,
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
