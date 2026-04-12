use axum::{
    extract::Path,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::{fs, path::Path as FsPath};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/*path", get(handler))
        .nest_service("/assets", ServeDir::new("assets"))
        .nest_service("/components", ServeDir::new("components"));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .unwrap();

    println!("rodando em http://localhost:8000");

    axum::serve(listener, app).await.unwrap();
}

async fn handler(Path(path): Path<String>) -> impl IntoResponse {
    let clean_path = if path == "/" || path.is_empty() {
        "index".to_string()
    } else {
        path.trim_start_matches('/').to_string()
    };

    let try_files = vec![
        format!("{}.html", clean_path),
        format!("{}/index.html", clean_path),
    ];

    for file in try_files {
        println!("tentando abrir: {}", file);

        if FsPath::new(&file).exists() {
            match fs::read_to_string(&file) {
                Ok(content) => return Html(content).into_response(),
                Err(_) => continue,
            }
        }
    }

    Html("<h1>404 Not Found</h1>".to_string()).into_response()
}
async fn index() -> impl IntoResponse {
    match fs::read_to_string("index.html") {
        Ok(content) => Html(content).into_response(),
        Err(_) => Html("<h1>index.html não encontrado</h1>".to_string()).into_response(),
    }
}
