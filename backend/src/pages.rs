use axum::{
    extract::Path,
    response::{Html, IntoResponse},
};
use std::fs;

use crate::utils::montar_pagina;

// Página inicial (index.html já é um documento completo, não precisa de wrapper)
pub async fn inicio() -> impl IntoResponse {
    match fs::read_to_string("index.html") {
        Ok(html) => Html(html).into_response(),
        Err(_)   => Html("<h1>index.html não encontrado</h1>".to_string()).into_response(),
    }
}

pub async fn pagina_registro() -> impl IntoResponse {
    servir_pagina("u/register.html", "Criar conta - Santuario Thinkpad")
}

pub async fn pagina_login() -> impl IntoResponse {
    servir_pagina("u/login.html", "Login - Santuario Thinkpad")
}

pub async fn pagina_forum() -> impl IntoResponse {
    servir_pagina("s.html", "Forum - Santuario Thinkpad")
}

// O template de post é o mesmo para todos os posts; o JS usa a URL para saber qual carregar
pub async fn pagina_post(Path(_id): Path<i32>) -> impl IntoResponse {
    servir_pagina("s/post.html", "Post - Santuario Thinkpad")
}

// Serve qualquer outro arquivo HTML pelo caminho da URL
pub async fn pagina_estatica(Path(caminho): Path<String>) -> impl IntoResponse {
    let arquivo = format!("{}.html", caminho);
    match fs::read_to_string(&arquivo) {
        Ok(html) => Html(montar_pagina("Santuario Thinkpad", &html)).into_response(),
        Err(_)   => Html(montar_pagina("404", "<main><article><h1>404 — Página não encontrada</h1></article></main>")).into_response(),
    }
}

// Lê um arquivo HTML parcial e o envolve no template completo
fn servir_pagina(arquivo: &str, titulo: &str) -> impl IntoResponse {
    match fs::read_to_string(arquivo) {
        Ok(html) => Html(montar_pagina(titulo, &html)),
        Err(_)   => Html(montar_pagina("Erro", &format!(
            "<main><article><h1>{} não encontrado</h1></article></main>", arquivo
        ))),
    }
}
