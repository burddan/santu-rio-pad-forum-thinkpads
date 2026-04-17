use axum::{
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;

use crate::{
    models::{ApiResponse, CommentInput, CommentItem, PostDetail, PostSummary},
    utils::get_session_token,
    AppState,
};

pub async fn list_posts(State(state): State<AppState>) -> Response {
    let rows = sqlx::query_as::<_, (i32, String, String, String, Option<String>, String, i64)>(
        r#"SELECT p.id, p.username, p.title, p.content, p.image_path,
                  TO_CHAR(p.created_at AT TIME ZONE 'America/Sao_Paulo', 'DD/MM/YYYY HH24:MI') AS created_at,
                  COUNT(c.id) AS comment_count
           FROM posts p
           LEFT JOIN comments c ON c.post_id = p.id
           GROUP BY p.id
           ORDER BY p.created_at DESC"#,
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => {
            let posts: Vec<PostSummary> = rows
                .into_iter()
                .map(|(id, username, title, content, image_path, created_at, comment_count)| {
                    PostSummary { id, username, title, content, image_path, created_at, comment_count }
                })
                .collect();
            (StatusCode::OK, Json(posts)).into_response()
        }
        Err(e) => {
            eprintln!("list_posts error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse { ok: false, message: "Erro ao buscar posts.".into() }),
            )
                .into_response()
        }
    }
}

pub async fn create_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
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

    let session = sqlx::query_as::<_, (String,)>("SELECT username FROM sessions WHERE token = $1")
        .bind(&token)
        .fetch_optional(&state.db)
        .await;

    let username = match session {
        Ok(Some((u,))) => u,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse { ok: false, message: "Sessão inválida.".into() }),
            )
                .into_response();
        }
    };

    let mut title = String::new();
    let mut content = String::new();
    let mut image_path: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "title" => title = field.text().await.unwrap_or_default(),
            "content" => content = field.text().await.unwrap_or_default(),
            "image" => {
                let ext = field
                    .file_name()
                    .and_then(|f| f.rsplit('.').next())
                    .unwrap_or("jpg")
                    .to_string();
                let data = field.bytes().await.unwrap_or_default();
                if !data.is_empty() {
                    let save_name = format!("{}.{}", Uuid::new_v4(), ext);
                    let path = format!("uploads/{}", save_name);
                    if tokio::fs::write(&path, &data).await.is_ok() {
                        image_path = Some(format!("/uploads/{}", save_name));
                    }
                }
            }
            _ => {}
        }
    }

    let title = title.trim().to_string();
    let content = content.trim().to_string();

    if title.is_empty() || content.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse { ok: false, message: "Título e conteúdo são obrigatórios.".into() }),
        )
            .into_response();
    }

    let result = sqlx::query(
        "INSERT INTO posts (username, title, content, image_path) VALUES ($1, $2, $3, $4)",
    )
    .bind(&username)
    .bind(&title)
    .bind(&content)
    .bind(&image_path)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse { ok: true, message: "Post criado!".into() }),
        )
            .into_response(),
        Err(e) => {
            eprintln!("create_post error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse { ok: false, message: "Erro ao criar post.".into() }),
            )
                .into_response()
        }
    }
}

pub async fn get_post(State(state): State<AppState>, Path(id): Path<i32>) -> Response {
    let post = sqlx::query_as::<_, (i32, String, String, String, Option<String>, String)>(
        r#"SELECT id, username, title, content, image_path,
                  TO_CHAR(created_at AT TIME ZONE 'America/Sao_Paulo', 'DD/MM/YYYY HH24:MI') AS created_at
           FROM posts WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await;

    let (post_id, username, title, content, image_path, created_at) = match post {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse { ok: false, message: "Post não encontrado.".into() }),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse { ok: false, message: "Erro.".into() }),
            )
                .into_response();
        }
    };

    let comments = sqlx::query_as::<_, (i32, String, String, String)>(
        r#"SELECT id, username, content,
                  TO_CHAR(created_at AT TIME ZONE 'America/Sao_Paulo', 'DD/MM/YYYY HH24:MI') AS created_at
           FROM comments WHERE post_id = $1 ORDER BY created_at ASC"#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(id, username, content, created_at)| CommentItem { id, username, content, created_at })
    .collect();

    (
        StatusCode::OK,
        Json(PostDetail { id: post_id, username, title, content, image_path, created_at, comments }),
    )
        .into_response()
}

pub async fn add_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(post_id): Path<i32>,
    Json(payload): Json<CommentInput>,
) -> Response {
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

    let session = sqlx::query_as::<_, (String,)>("SELECT username FROM sessions WHERE token = $1")
        .bind(&token)
        .fetch_optional(&state.db)
        .await;

    let username = match session {
        Ok(Some((u,))) => u,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse { ok: false, message: "Sessão inválida.".into() }),
            )
                .into_response();
        }
    };

    let content = payload.content.trim().to_string();
    if content.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse { ok: false, message: "Comentário vazio.".into() }),
        )
            .into_response();
    }

    let result = sqlx::query(
        "INSERT INTO comments (post_id, username, content) VALUES ($1, $2, $3)",
    )
    .bind(post_id)
    .bind(&username)
    .bind(&content)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse { ok: true, message: "Comentário adicionado.".into() }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse { ok: false, message: "Erro ao comentar.".into() }),
        )
            .into_response(),
    }
}
