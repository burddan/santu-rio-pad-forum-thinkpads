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

    println!("funcionando banco");

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

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS posts (
            id         SERIAL PRIMARY KEY,
            username   TEXT        NOT NULL,
            title      TEXT        NOT NULL,
            content    TEXT        NOT NULL,
            image_path TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(&db)
    .await
    .expect("falha ao criar tabela posts");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS comments (
            id         SERIAL PRIMARY KEY,
            post_id    INTEGER     NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            username   TEXT        NOT NULL,
            content    TEXT        NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(&db)
    .await
    .expect("falha ao criar tabela comments");

    tokio::fs::create_dir_all("uploads").await.ok();

    let state = AppState { db };

    let app = Router::new()
        .route("/",                       get(pages::index))
        .route("/u/register",             get(pages::register_page).post(auth::register))
        .route("/u/login",                get(pages::login_page).post(auth::login))
        .route("/logout",                 post(auth::logout))
        .route("/me",                     get(auth::me))
        .route("/users",                  get(users::list_users))
        .route("/s",                      get(pages::forum_page))
        .route("/s/:id",                  get(pages::post_page))
        .route("/api/posts",              get(forum::list_posts).post(forum::create_post))
        .route("/api/posts/:id",          get(forum::get_post))
        .route("/api/posts/:id/comments", post(forum::add_comment))
        .route("/*path",                  get(pages::static_handler))
        .nest_service("/assets",          ServeDir::new("assets"))
        .nest_service("/components",      ServeDir::new("components"))
        .nest_service("/uploads",         ServeDir::new("uploads"))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .unwrap();

    println!("rodando em http://localhost:8000");

    axum::serve(listener, app).await.unwrap();
}
