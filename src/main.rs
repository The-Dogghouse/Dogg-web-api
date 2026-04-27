use axum::{routing::get, Json, Router};

use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx;
use sqlx::sqlite;
use sqlx::SqlitePool;

use tower_http::cors::CorsLayer;

use axum::extract::State;
use axum::http::StatusCode;
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use chrono::{DateTime, Utc};
use sqlx::types::Uuid;
use std::str::FromStr;

use validator::{Validate, ValidationErrors};

enum AppError {
    Database(sqlx::Error),
    BadRequest(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Database(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error {}", msg).to_string(),
            ),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        (status, Json(json!({"error": message}))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Database(e)
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(value: ValidationErrors) -> Self {
        AppError::BadRequest(value.to_string())
    }
}

#[derive(sqlx::FromRow, Serialize)]
struct LeaderboardEntry {
    id: Uuid,
    name: String,
    clicks: i64,
    created_at: DateTime<Utc>,
}

async fn get_leaderboard(
    State(pool): State<SqlitePool>,
) -> Result<Json<Vec<LeaderboardEntry>>, AppError> {
    let results = sqlx::query_as!(
        LeaderboardEntry,
        r#"select id as "id: Uuid",
       name,
       clicks,
       created_at as "created_at: DateTime<Utc>"
from leaderboard
order by clicks desc;"#
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(results))
}

#[derive(Deserialize, Validate)]
struct AddLeaderboardEntry {
    id: Uuid,
    #[validate(length(min = 1, max = 128))]
    name: String,
    #[validate(range(min = 1))]
    clicks: i64,
}

#[derive(Serialize)]
struct AddLeaderboardEntryResponse {
    id: Uuid,
}

async fn add_leaderboard_entry(
    State(pool): State<SqlitePool>,
    Json(body): Json<AddLeaderboardEntry>,
) -> Result<Json<AddLeaderboardEntryResponse>, AppError> {
    body.validate()?;
    let now = Utc::now();
    sqlx::query(
        "insert or replace into leaderboard (id, name, clicks, created_at) values (?, ?, ?, ?);",
    )
    .bind(&body.id)
    .bind(&body.name)
    .bind(&body.clicks)
    .bind(now)
    .execute(&pool)
    .await?;

    Ok(Json(AddLeaderboardEntryResponse { id: body.id }))
}

#[tokio::main]
async fn main() {
    let options = sqlite::SqliteConnectOptions::from_str("sqlite:dogg-web.db")
        .unwrap()
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options).await.unwrap();

    sqlx::migrate!("db/migrations").run(&pool).await.unwrap();

    let cors = CorsLayer::new()
        .allow_origin("https://dogg-house.net".parse::<HeaderValue>().unwrap())
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let app = Router::new()
        .route("/api/leaderboard", get(get_leaderboard))
        .route("/api/leaderboard", post(add_leaderboard_entry))
        .layer(cors)
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
