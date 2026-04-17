use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RegisterInput {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct ApiResponse {
    pub ok: bool,
    pub message: String,
}

#[derive(Serialize)]
pub struct UserEntry {
    pub username: String,
}

#[derive(Serialize)]
pub struct PostSummary {
    pub id: i32,
    pub username: String,
    pub title: String,
    pub content: String,
    pub image_path: Option<String>,
    pub created_at: String,
    pub comment_count: i64,
}

#[derive(Serialize)]
pub struct CommentItem {
    pub id: i32,
    pub username: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct PostDetail {
    pub id: i32,
    pub username: String,
    pub title: String,
    pub content: String,
    pub image_path: Option<String>,
    pub created_at: String,
    pub comments: Vec<CommentItem>,
}

#[derive(Deserialize)]
pub struct CommentInput {
    pub content: String,
}
