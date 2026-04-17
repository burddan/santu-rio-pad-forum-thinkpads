use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;

use crate::{
    models::{ApiResponse, LoginInput, RegisterInput},
    utils::get_session_token,
    AppState,
};

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterInput>,
) -> Response {
    let username = payload.username.trim().to_string();
    let password = payload.password.trim().to_string();

    if username.is_empty() || password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse { ok: false, message: "Usuário e senha são obrigatórios.".into() }),
        )
            .into_response();
    }

    if password.len() < 6 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse { ok: false, message: "A senha deve ter pelo menos 6 caracteres.".into() }),
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
            Json(ApiResponse { ok: true, message: "Conta criada com sucesso!".into() }),
        )
            .into_response(),
        Err(e) if e.to_string().contains("duplicate key") => (
            StatusCode::CONFLICT,
            Json(ApiResponse { ok: false, message: "Nome de usuário já existe.".into() }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse { ok: false, message: "Erro interno ao registrar.".into() }),
        )
            .into_response(),
    }
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginInput>,
) -> Response {
    let username = payload.username.trim().to_string();
    let password = payload.password.trim().to_string();

    let row = sqlx::query_as::<_, (i32, String, String)>(
        "SELECT id, username, password FROM users WHERE username = $1",
    )
    .bind(&username)
    .fetch_optional(&state.db)
    .await;

    let (user_id, db_username, hash): (i32, String, String) = match row {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse { ok: false, message: "Usuário ou senha incorretos.".into() }),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse { ok: false, message: "Erro interno.".into() }),
            )
                .into_response();
        }
    };

    if !bcrypt::verify(&password, &hash).unwrap_or(false) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse { ok: false, message: "Usuário ou senha incorretos.".into() }),
        )
            .into_response();
    }

    let token = Uuid::new_v4().to_string();
    if let Err(e) = sqlx::query(
        "INSERT INTO sessions (token, user_id, username) VALUES ($1, $2, $3)",
    )
    .bind(&token)
    .bind(user_id)
    .bind(&db_username)
    .execute(&state.db)
    .await
    {
        eprintln!("ERRO AO CRIAR SESSÃO: {:?}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse { ok: false, message: "Erro ao criar sessão.".into() }),
        )
            .into_response();
    }

    let cookie = format!(
        "session={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=604800",
        token
    );
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie.parse().unwrap());

    (
        StatusCode::OK,
        headers,
        Json(ApiResponse { ok: true, message: format!("Bem-vindo, {}!", db_username) }),
    )
        .into_response()
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
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
        Json(ApiResponse { ok: true, message: "Logout realizado.".into() }),
    )
        .into_response()
}

pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let token = match get_session_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse { ok: false, message: "Não autenticado.".into() }),
            )
                .into_response();
        }
    };

    let row = sqlx::query_as::<_, (String,)>("SELECT username FROM sessions WHERE token = $1")
        .bind(&token)
        .fetch_optional(&state.db)
        .await;

    match row {
        Ok(Some((username,))) => (
            StatusCode::OK,
            Json(ApiResponse { ok: true, message: username }),
        )
            .into_response(),
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse { ok: false, message: "Sessão inválida.".into() }),
        )
            .into_response(),
    }
}
