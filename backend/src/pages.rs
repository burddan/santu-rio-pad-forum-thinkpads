use axum::{
    extract::Path,
    response::{Html, IntoResponse},
};
use std::fs;

use crate::utils::wrap_page;

pub async fn index() -> impl IntoResponse {
    match fs::read_to_string("index.html") {
        Ok(content) => Html(content).into_response(),
        Err(_) => Html("<h1>index.html não encontrado</h1>".to_string()).into_response(),
    }
}

pub async fn register_page() -> impl IntoResponse {
    match fs::read_to_string("u/register.html") {
        Ok(content) => Html(wrap_page("Criar conta - Santuario Thinkpad", &content)),
        Err(_) => Html(wrap_page("Erro", "<main><article><h1>register.html não encontrado</h1></article></main>")),
    }
}

pub async fn login_page() -> impl IntoResponse {
    match fs::read_to_string("u/login.html") {
        Ok(content) => Html(wrap_page("Login - Santuario Thinkpad", &content)),
        Err(_) => Html(wrap_page("Erro", "<main><article><h1>login.html não encontrado</h1></article></main>")),
    }
}

pub async fn forum_page() -> impl IntoResponse {
    match fs::read_to_string("s.html") {
        Ok(content) => Html(wrap_page("Forum - Santuario Thinkpad", &content)),
        Err(_) => Html(wrap_page("Erro", "<main><article><h1>s.html não encontrado</h1></article></main>")),
    }
}

pub async fn post_page(Path(_id): Path<i32>) -> impl IntoResponse {
    match fs::read_to_string("s/post.html") {
        Ok(content) => Html(wrap_page("Post - Santuario Thinkpad", &content)),
        Err(_) => Html(wrap_page("Erro", "<main><article><h1>post.html não encontrado</h1></article></main>")),
    }
}

pub async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    let file_path = format!("{}.html", path);
    match fs::read_to_string(&file_path) {
        Ok(content) => Html(wrap_page("Santuario Thinkpad", &content)).into_response(),
        Err(_) => Html(wrap_page("404", "<main><article><h1>404 Not Found</h1></article></main>")).into_response(),
    }
}
