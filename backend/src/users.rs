use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::{models::{Resposta, UsuarioItem}, AppState};

// Retorna a lista de todos os usuários cadastrados
pub async fn listar_usuarios(State(estado): State<AppState>) -> Response {
    let linhas = sqlx::query_as::<_, (String,)>(
        "SELECT usuario FROM usuarios ORDER BY id ASC",
    )
    .fetch_all(&estado.db)
    .await;

    match linhas {
        Ok(linhas) => {
            let usuarios: Vec<UsuarioItem> = linhas
                .into_iter()
                .map(|(usuario,)| UsuarioItem { usuario })
                .collect();
            (StatusCode::OK, Json(usuarios)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Resposta { ok: false, mensagem: "Erro ao buscar usuários.".into() }),
        )
            .into_response(),
    }
}
