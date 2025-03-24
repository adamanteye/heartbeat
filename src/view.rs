use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::post,
};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Deserialize)]
pub(super) struct HeartbeatReport {
    source: String,
    event: String,
    note: String,
    token: String,
}

#[derive(Eq)]
struct HeartbeatRecord {
    time: DateTime<Utc>,
    source: String,
    event: String,
    note: String,
}

impl PartialEq for HeartbeatRecord {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl PartialOrd for HeartbeatRecord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeartbeatRecord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time).reverse()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub conn: Arc<Mutex<Connection>>,
    pub secret: Arc<String>,
}

pub(super) enum AppError {
    Unauthorized(String),
    Server(String),
}

#[derive(Serialize)]
pub(super) struct AppErrorResponse {
    msg: String,
    ver: &'static str,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            Self::Server(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
        };
        (
            status,
            Json(AppErrorResponse {
                msg: message,
                ver: env!("CARGO_PKG_VERSION"),
            }),
        )
            .into_response()
    }
}

pub(super) async fn render(State(state): State<AppState>) -> Result<Response, AppError> {
    let lock = state.conn.lock().await;
    let mut stmt = lock
        .prepare("SELECT time, source, event, note FROM record")
        .map_err(|e| AppError::Server(format!("{e}")))?;
    let iter = stmt
        .query_map([], |row| {
            Ok(HeartbeatRecord {
                time: {
                    let time: String = row.get(0)?;
                    DateTime::parse_from_rfc3339(&time).unwrap().into()
                },
                source: row.get(1)?,
                event: row.get(2)?,
                note: row.get(3)?,
            })
        })
        .map_err(|e| AppError::Server(format!("{e}")))?;
    let mut res = Vec::new();
    for record in iter {
        res.push(record.map_err(|e| AppError::Server(format!("{e}")))?);
    }
    res.sort_unstable();
    let mut table = Vec::new();
    for res in res {
        table.push(format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            res.time.format("%Y-%m-%d %H:%M"),
            res.event,
            res.source,
            res.note,
        ));
    }
    let html = format!(
        r#"<!doctype html><html lang=zh><meta charset=UTF-8><link rel=icon type=image/x-icon href=https://note.adamanteye.cc/assets/favicon.ico><link rel=preload as=style crossorigin href=https://static.zeoseven.com/zsft/442/main/result.css onload='this.rel="stylesheet"' onerror='this.href="https://static-host.zeoseven.com/zsft/442/main/result.css"'><noscript><link rel=stylesheet href=https://static.zeoseven.com/zsft/442/main/result.css></noscript><link rel=stylesheet href=https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.0.0/css/all.min.css><title>Adamanteye's Heartbeats</title><style>:root{{--primary-color:#d20f39;--secondary-color:#ea76cb;--teal-color:#179299;--url-color:#e39067;--text-color:#4c4f69;--mauve-color:#8839ef;--background-color:#fffefa;--font-family:"Maple Mono NF CN"}}body{{font-family:var(--font-family);font-weight:400;background-color:var(--background-color);color:var(--text-color)}}th{{color:var(--primary-color)}}#main-grid{{display:grid;grid-template-columns:1fr;grid-template-areas:"header" "main" "footer";min-height:100vh;grid-template-rows:auto 1fr auto}}header{{grid-area:header}}main{{grid-area:main}}table{{table-layout:fixed;text-align:left;width:100%;border-collapse:separate;border-spacing:0 .4em;margin-bottom:1em}}thead th:nth-child(1){{width:15%}}thead th:nth-child(2){{width:40%}}thead th:nth-child(3){{width:25%}}thead th:nth-child(4){{width:20%}}footer{{grid-area:footer;margin:auto;margin-top:2em}}tr a{{color:var(--url-color)}}footer>a{{color:var(--secondary-color)}}</style><body id=main-grid><header><h1>Adamanteye's Heartbeats</h1></header><main><table><thead><tr><th scope=col>Time [UTC]<th scope=col>Event<th scope=col>Source<th scope=col>Note<tbody>{}</table></main><footer><i class="fa-regular fa-copyright"></i>
        <b>2025</b>
        <b>adamanteye</b>
        <a rel=license href=http://creativecommons.org/licenses/by-sa/4.0/>
        <b>CC BY-SA 4.0</b></a></footer>"#,
        table.join("")
    );
    Ok(Html(html).into_response())
}

pub(super) async fn receive(
    State(state): State<AppState>,
    Json(data): Json<HeartbeatReport>,
) -> Result<Response, AppError> {
    log::debug!("receiving signal");
    if data.token == *state.secret {
        let time = Utc::now();
        state
            .conn
            .lock()
            .await
            .execute(
                "INSERT INTO record (time, source, event, note) VALUES (?1, ?2, ?3, ?4)",
                (
                    time.to_rfc3339(),
                    data.source.clone(),
                    data.event.clone(),
                    data.note,
                ),
            )
            .map_err(|e| AppError::Server(format!("{e}")))?;
        log::info!("received {} heartbeat form {}", data.event, data.source);
        Ok(StatusCode::CREATED.into_response())
    } else {
        Err(AppError::Unauthorized(
            "invalid authentication token".to_string(),
        ))
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", post(receive).get(render))
        .with_state(state)
}
