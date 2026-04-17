use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::{models::{ApiResponse, UserEntry}, AppState};

pub async fn list_users(State(state): State<AppState>) -> Response {
    let rows = sqlx::query_as::<_, (String,)>("SELECT username FROM users ORDER BY id ASC")
        .fetch_all(&state.db)
        .await;

    match rows {
        Ok(rows) => {
            let users: Vec<UserEntry> = rows
                .into_iter()
                .map(|(username,)| UserEntry { username })
                .collect();
            (StatusCode::OK, Json(users)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse { ok: false, message: "Erro ao buscar usuários.".into() }),
        )
            .into_response(),
    }
}
