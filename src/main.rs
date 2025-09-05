use axum::{
  extract::{Path, State},
  http::StatusCode,
  response::{IntoResponse, Response},
  routing::{get, post},
  Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Postgres, Pool};
use std::{net::SocketAddr, time::Duration};
use tokio::time::{sleep, timeout};

#[derive(Clone)]
struct AppState {
  pool: Pool<Postgres>,
}

#[derive(Serialize)]
struct Msg<'a> {
  message: &'a str,
}

#[derive(Serialize)]
struct ErrMsg<'a> {
  error: &'a str,
  #[serde(skip_serializing_if = "Option::is_none")]
  detail: Option<String>,
}

#[derive(Deserialize)]
struct CreateUserReq {
  username: String,
  email: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct User {
  user_id: i32,   // ถ้าใช้ BIGSERIAL ให้เปลี่ยนเป็น i64
  username: String,
  email: String,
}

const QUERY_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  dotenvy::dotenv().ok();

  // แนะนำให้ default เป็น "postgres" (ชื่อ service ใน compose) — ปรับตามของคุณได้
  let db_host = env_or("DB_HOST", "container_postgresql");
  let db_user = env_or("DB_USER", "testuser");
  let db_pass = env_or("DB_PASSWORD", "testpass");
  let db_name = env_or("DB_NAME", "testdb");
  let db_port = env_or("DB_PORT", "5432");
  let http_port = env_or("PORT", "3000");

  let dsn = format!(
    "postgres://{user}:{pass}@{host}:{port}/{name}?sslmode=disable",
    user = urlencoding::encode(&db_user),
    pass = urlencoding::encode(&db_pass),
    host = db_host,
    port = db_port,
    name = db_name,
  );

  println!(
    "Booting API... host={} db={} port={} (will retry DB connect if needed)",
    db_host, db_name, db_port
  );

  let pool = connect_with_retry(&dsn, 20, Duration::from_secs(3)).await?;
  println!("DB connected.");

  // สร้างตาราง (กันลืม init script) — เอาออกได้ถ้าใช้ initdb อยู่แล้ว
  ensure_schema(&pool).await?;
  println!("Schema ensured.");

  // sanity check
  sqlx::query("SELECT 1").execute(&pool).await?;

  let state = AppState { pool };

  let app = Router::new()
    .route("/", get(handle_root))
    .route("/users", post(create_user))
    .route("/users/:id", get(get_user))
    .fallback(fallback_404)
    .with_state(state);

  let addr: SocketAddr = format!("0.0.0.0:{}", http_port).parse()?;
  println!("Server listening on http://{}", addr);
  axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

  Ok(())
}

async fn connect_with_retry(dsn: &str, max_attempts: u32, base_delay: Duration) -> anyhow::Result<Pool<Postgres>> {
  let mut attempt = 1;
  loop {
    match PgPoolOptions::new()
      .max_connections(10)
      .acquire_timeout(Duration::from_secs(60))
      .idle_timeout(Duration::from_secs(600))
      .max_lifetime(Duration::from_secs(1800))
      .test_before_acquire(true)
      .connect(dsn)
      .await
    {
      Ok(pool) => return Ok(pool),
      Err(e) if attempt < max_attempts => {
        eprintln!("DB connect failed (attempt {}/{}): {} — retrying...", attempt, max_attempts, e);
        // backoff: 3s, 6s, 9s, ...
        sleep(base_delay * attempt).await;
        attempt += 1;
      }
      Err(e) => {
        eprintln!("DB connect failed (attempt {}/{}): {}", attempt, max_attempts, e);
        return Err(e.into());
      }
    }
  }
}

async fn ensure_schema(pool: &Pool<Postgres>) -> anyhow::Result<()> {
  // SERIAL => i32 ได้; ถ้าจะใช้ BIGSERIAL ให้แก้ struct เป็น i64
  let sql = r#"
    CREATE TABLE IF NOT EXISTS public.users (
      user_id  SERIAL PRIMARY KEY,
      username VARCHAR(100) NOT NULL,
      email    VARCHAR(150) NOT NULL UNIQUE
    );
  "#;
  sqlx::query(sql).execute(pool).await?;
  Ok(())
}

async fn handle_root() -> impl IntoResponse {
  (StatusCode::OK, Json(Msg { message: "Hello World from Rust (PostgreSQL)" }))
}

async fn create_user(
  State(state): State<AppState>,
  Json(payload): Json<CreateUserReq>,
) -> Response {
  if payload.username.trim().is_empty() || payload.email.trim().is_empty() {
    return (
      StatusCode::BAD_REQUEST,
      Json(ErrMsg { error: "username and email are required", detail: None })
    ).into_response();
  }

  let fut = sqlx::query_scalar::<_, i32>(
    "INSERT INTO users (username, email) VALUES ($1, $2) RETURNING user_id",
  )
  .bind(&payload.username)
  .bind(&payload.email)
  .fetch_one(&state.pool);

  match timeout(QUERY_TIMEOUT, fut).await {
    Err(_) => (StatusCode::GATEWAY_TIMEOUT, Json(ErrMsg { error: "Database timeout", detail: None })).into_response(),
    Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrMsg { error: "Database error", detail: Some(e.to_string()) })).into_response(),
    Ok(Ok(id)) => (
      StatusCode::CREATED,
      Json(serde_json::json!({ "message": "User created successfully", "user_id": id }))
    ).into_response(),
  }
}

async fn get_user(
  State(state): State<AppState>,
  Path(id): Path<i32>, // ถ้าใช้ BIGSERIAL ให้เปลี่ยนเป็น i64
) -> Response {
  let fut = sqlx::query_as::<_, User>(
    "SELECT user_id, username, email FROM users WHERE user_id = $1",
  )
  .bind(id)
  .fetch_optional(&state.pool);

  match timeout(QUERY_TIMEOUT, fut).await {
    Err(_) => (StatusCode::GATEWAY_TIMEOUT, Json(ErrMsg { error: "Database timeout", detail: None })).into_response(),
    Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrMsg { error: "Database error", detail: Some(e.to_string()) })).into_response(),
    Ok(Ok(None)) => (StatusCode::NOT_FOUND, Json(ErrMsg { error: "User not found", detail: None })).into_response(),
    Ok(Ok(Some(user))) => (StatusCode::OK, Json(user)).into_response(),
  }
}

async fn fallback_404() -> impl IntoResponse {
  (StatusCode::NOT_FOUND, Json(ErrMsg { error: "Not Found", detail: None }))
}

fn env_or(key: &str, default: &str) -> String {
  std::env::var(key).unwrap_or_else(|_| default.to_string())
}
