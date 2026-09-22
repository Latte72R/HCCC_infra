use axum::{
    extract::Extension,
    http::header::CONTENT_TYPE,
    http::{HeaderValue, Method, StatusCode},
    routing, Json, Router,
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

use crate::controllers::{accounts, admin, problems, submissions, users};
use crate::database::{self, RepositoryProvider};
use crate::entities::Ranking;
use crate::request;
use crate::services;

/// Root of router.
/// It is also setting the allowed origins for CORS.
///
/// # Panics
/// When url parse failed.
pub async fn app() -> Router {
    let allowed_origins = [
        "http://localhost:3000".parse::<HeaderValue>().unwrap(),
        "https://hccc.latte72.net".parse::<HeaderValue>().unwrap(),
    ];

    let cors_layer = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::HEAD,
            Method::OPTIONS,
        ])
        .allow_headers([CONTENT_TYPE]);
    let database_layer = database::layer().await;
    let session_layer = request::layer().await;

    Router::new()
        .route("/", routing::get(get))
        .route("/healthz", routing::get(health))
        .route("/readyz", routing::get(ready))
        .route("/api/admin/overview", routing::get(admin::overview))
        .route(
            "/api/admin/contest",
            routing::get(admin::get_contest_period).put(admin::update_contest_period),
        )
        .route(
            "/api/admin/submissions/:id/judgement",
            routing::put(admin::correct_judgement),
        )
        .route("/api/login", routing::post(accounts::login))
        .route("/api/logout", routing::post(accounts::logout))
        .route("/api/register", routing::post(accounts::register))
        .route("/api/ranking", routing::get(ranking))
        .nest("/api/users", users::user())
        .nest("/api/problems", problems::problem())
        .nest("/api/submissions", submissions::submissions())
        .layer(cors_layer)
        .layer(database_layer)
        .layer(session_layer)
}

/// Return 404 to `/`.
async fn get() -> StatusCode {
    StatusCode::NOT_FOUND
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn ready(Extension(pool): Extension<PgPool>) -> StatusCode {
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

/// Return `/api/ranking` api.
async fn ranking(Extension(repository_provider): Extension<RepositoryProvider>) -> Json<Ranking> {
    tracing::debug!("/api/ranking");
    let user_repo = repository_provider.user();
    Json(services::get_ranking(&user_repo).await)
}
