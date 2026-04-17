use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::fs;
use tower_http::services::ServeDir;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: PgPool,
}

// -------------------- STRUCTS --------------------

#[derive(Deserialize)]
struct RegisterInput {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginInput {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct ApiResponse {
    ok: bool,
    message: String,
}

// -------------------- MAIN --------------------

#[tokio::main]
async fn main() {
    let db = PgPool::connect("postgres://root:1234@localhost:5432/santuario")
        .await
        .expect("falha ao conectar ao banco");

    println!("funcionando banco");

    // Cria tabelas
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id       SERIAL PRIMARY KEY,
            username TEXT    NOT NULL UNIQUE,
            password TEXT    NOT NULL
        )",
    )
    .execute(&db)
    .await
    .expect("falha ao criar tabela users");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sessions (
            token    TEXT    PRIMARY KEY,
            user_id  INTEGER NOT NULL,
            username TEXT    NOT NULL
        )",
    )
    .execute(&db)
    .await
    .expect("falha ao criar tabela sessions");

    let state = AppState { db };

    let app = Router::new()
        .route("/api/register", post(register))
        .route("/api/login",    post(login))
        .route("/api/logout",   post(logout))
        .route("/api/me",       get(me))
        .route("/*path",        get(static_handler))
        .nest_service("/assets",     ServeDir::new("assets"))
        .nest_service("/components", ServeDir::new("components"))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .unwrap();

    println!("rodando em http://localhost:8000");

    axum::serve(listener, app).await.unwrap();
}

// -------------------- REGISTER --------------------

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterInput>,
) -> Response {
    let username = payload.username.trim().to_string();
    let password = payload.password.trim().to_string();

    if username.is_empty() || password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                ok: false,
                message: "Usuário e senha são obrigatórios.".into(),
            }),
        )
            .into_response();
    }

    if password.len() < 6 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                ok: false,
                message: "A senha deve ter pelo menos 6 caracteres.".into(),
            }),
        )
            .into_response();
    }

    let hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST).expect("falha ao gerar hash");

    let result = sqlx::query("INSERT INTO users (username, password) VALUES ($1, $2)")
        .bind(&username)
        .bind(&hash)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse {
                ok: true,
                message: "Conta criada com sucesso!".into(),
            }),
        )
            .into_response(),
        Err(e) if e.to_string().contains("duplicate key") => (
            StatusCode::CONFLICT,
            Json(ApiResponse {
                ok: false,
                message: "Nome de usuário já existe.".into(),
            }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse {
                ok: false,
                message: "Erro interno ao registrar.".into(),
            }),
        )
            .into_response(),
    }
}

// -------------------- LOGIN --------------------

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginInput>,
) -> Response {
    let username = payload.username.trim().to_string();
    let password = payload.password.trim().to_string();

    let row = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, username, password FROM users WHERE username = $1",
    )
    .bind(&username)
    .fetch_optional(&state.db)
    .await;

    let (user_id, db_username, hash) = match row {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse {
                    ok: false,
                    message: "Usuário ou senha incorretos.".into(),
                }),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    ok: false,
                    message: "Erro interno.".into(),
                }),
            )
                .into_response();
        }
    };

    let valid = bcrypt::verify(&password, &hash).unwrap_or(false);
    if !valid {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse {
                ok: false,
                message: "Usuário ou senha incorretos.".into(),
            }),
        )
            .into_response();
    }

    let token = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sessions (token, user_id, username) VALUES ($1, $2, $3)")
        .bind(&token)
        .bind(user_id)
        .bind(&db_username)
        .execute(&state.db)
        .await
        .expect("falha ao criar sessão");

    let cookie = format!(
        "session={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=604800",
        token
    );

    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie.parse().unwrap());

    (
        StatusCode::OK,
        headers,
        Json(ApiResponse {
            ok: true,
            message: format!("Bem-vindo, {}!", db_username),
        }),
    )
        .into_response()
}

// -------------------- LOGOUT --------------------

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(token) = get_session_token(&headers) {
        sqlx::query("DELETE FROM sessions WHERE token = $1")
            .bind(&token)
            .execute(&state.db)
            .await
            .ok();
    }

    let clear = "session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0";
    let mut h = HeaderMap::new();
    h.insert(header::SET_COOKIE, clear.parse().unwrap());

    (
        StatusCode::OK,
        h,
        Json(ApiResponse {
            ok: true,
            message: "Logout realizado.".into(),
        }),
    )
        .into_response()
}

// -------------------- ME --------------------

async fn me(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let token = match get_session_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse {
                    ok: false,
                    message: "Não autenticado.".into(),
                }),
            )
                .into_response();
        }
    };

    let row = sqlx::query_as::<_, (String,)>(
        "SELECT username FROM sessions WHERE token = $1"
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await;

    match row {
        Ok(Some((username,))) => (
            StatusCode::OK,
            Json(ApiResponse {
                ok: true,
                message: username,
            }),
        )
            .into_response(),
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse {
                ok: false,
                message: "Sessão inválida.".into(),
            }),
        )
            .into_response(),
    }
}

// -------------------- UTILS --------------------

fn get_session_token(headers: &HeaderMap) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookies.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("session=") {
            return Some(val.to_string());
        }
    }
    None
}

// -------------------- STATIC --------------------

async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    let path = if path.is_empty() {
        "index".to_string()
    } else {
        path
    };

    let file_path = format!("{}.html", path);

    match fs::read_to_string(&file_path) {
        Ok(content) => Html(content).into_response(),
        Err(_) => Html("<h1>404 Not Found</h1>").into_response(),
    }
}
