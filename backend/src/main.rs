mod auth;
mod forum;
mod models;
mod pages;
mod users;
mod utils;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

#[tokio::main]
async fn main() {
    let db = PgPool::connect("postgres://root:1234@localhost:5432/santuario")
        .await
        .expect("falha ao conectar ao banco");

    println!("banco conectado");

    criar_tabelas(&db).await;
    tokio::fs::create_dir_all("uploads").await.ok();

    let estado = AppState { db };

    let app = Router::new()
        // Páginas
        .route("/",                       get(pages::inicio))
        .route("/u/register",             get(pages::pagina_registro).post(auth::registrar))
        .route("/u/login",                get(pages::pagina_login).post(auth::entrar))
        .route("/s",                      get(pages::pagina_forum))
        .route("/s/:id",                  get(pages::pagina_post))
        // Autenticação
        .route("/logout",                 post(auth::sair))
        .route("/me",                     get(auth::quem_sou))
        // API
        .route("/users",                  get(users::listar_usuarios))
        .route("/api/posts",              get(forum::listar_posts).post(forum::criar_post))
        .route("/api/posts/:id",          get(forum::pegar_post))
        .route("/api/posts/:id/comments", post(forum::comentar))
        // Arquivos estáticos e catch-all
        .route("/*path",                  get(pages::pagina_estatica))
        .nest_service("/assets",          ServeDir::new("assets"))
        .nest_service("/components",      ServeDir::new("components"))
        .nest_service("/uploads",         ServeDir::new("uploads"))
        .with_state(estado);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000")
        .await
        .unwrap();

    println!("rodando em http://localhost:8000");

    axum::serve(listener, app).await.unwrap();
}

// Cria todas as tabelas caso ainda não existam
async fn criar_tabelas(db: &PgPool) {
    let tabelas = [
        "CREATE TABLE IF NOT EXISTS usuarios (
            id       SERIAL PRIMARY KEY,
            usuario  TEXT   NOT NULL UNIQUE,
            senha    TEXT   NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS sessoes (
            token      TEXT    PRIMARY KEY,
            id_usuario INTEGER NOT NULL,
            usuario    TEXT    NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS posts (
            id        SERIAL PRIMARY KEY,
            usuario   TEXT        NOT NULL,
            titulo    TEXT        NOT NULL,
            conteudo  TEXT        NOT NULL,
            imagem    TEXT,
            criado_em TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
        "CREATE TABLE IF NOT EXISTS comentarios (
            id        SERIAL  PRIMARY KEY,
            post_id   INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            usuario   TEXT        NOT NULL,
            conteudo  TEXT        NOT NULL,
            criado_em TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    ];

    for sql in &tabelas {
        sqlx::query(sql)
            .execute(db)
            .await
            .expect("falha ao criar tabela");
    }
}
