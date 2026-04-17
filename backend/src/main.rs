use axum::{
    extract::{Multipart, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::fs;
use tower_http::services::ServeDir;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: PgPool,
}

// -------------------- STRUCTS --------------------

#[derive(Deserialize)]
struct RegisterInput {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginInput {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct ApiResponse {
    ok: bool,
    message: String,
}

#[derive(Serialize)]
struct UserEntry {
    username: String,
}

#[derive(Serialize)]
struct PostSummary {
    id: i32,
    username: String,
    title: String,
    content: String,
    image_path: Option<String>,
    created_at: String,
    comment_count: i64,
}

#[derive(Serialize)]
struct CommentItem {
    id: i32,
    username: String,
    content: String,
    created_at: String,
}

#[derive(Serialize)]
struct PostDetail {
    id: i32,
    username: String,
    title: String,
    content: String,
    image_path: Option<String>,
    created_at: String,
    comments: Vec<CommentItem>,
}

#[derive(Deserialize)]
struct CommentInput {
    content: String,
}

// -------------------- MAIN --------------------

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
        .route("/",                          get(index))
        .route("/u/register",                get(register_page).post(register))
        .route("/u/login",                   get(login_page).post(login))
        .route("/logout",                    post(logout))
        .route("/me",                        get(me))
        .route("/users",                     get(list_users))
        .route("/s",                         get(forum_page))
        .route("/s/:id",                     get(post_page))
        .route("/api/posts",                 get(list_posts).post(create_post))
        .route("/api/posts/:id",             get(get_post))
        .route("/api/posts/:id/comments",    post(add_comment))
        .route("/*path",                     get(static_handler))
        .nest_service("/assets",             ServeDir::new("assets"))
        .nest_service("/components",         ServeDir::new("components"))
        .nest_service("/uploads",            ServeDir::new("uploads"))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .unwrap();

    println!("rodando em http://localhost:8000");

    axum::serve(listener, app).await.unwrap();
}

// -------------------- REGISTER --------------------

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterInput>,
) -> Response {
    let username = payload.username.trim().to_string();
    let password = payload.password.trim().to_string();

    if username.is_empty() || password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                ok: false,
                message: "Usuário e senha são obrigatórios.".into(),
            }),
        )
            .into_response();
    }

    if password.len() < 6 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                ok: false,
                message: "A senha deve ter pelo menos 6 caracteres.".into(),
            }),
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
            Json(ApiResponse {
                ok: true,
                message: "Conta criada com sucesso!".into(),
            }),
        )
            .into_response(),
        Err(e) if e.to_string().contains("duplicate key") => (
            StatusCode::CONFLICT,
            Json(ApiResponse {
                ok: false,
                message: "Nome de usuário já existe.".into(),
            }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse {
                ok: false,
                message: "Erro interno ao registrar.".into(),
            }),
        )
            .into_response(),
    }
}

// -------------------- LOGIN --------------------

async fn login(
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
                Json(ApiResponse {
                    ok: false,
                    message: "Usuário ou senha incorretos.".into(),
                }),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    ok: false,
                    message: "Erro interno.".into(),
                }),
            )
                .into_response();
        }
    };

    let valid = bcrypt::verify(&password, &hash).unwrap_or(false);
    if !valid {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse {
                ok: false,
                message: "Usuário ou senha incorretos.".into(),
            }),
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
        println!("ERRO AO CRIAR SESSÃO: {:?}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse {
                ok: false,
                message: "Erro ao criar sessão.".into(),
            }),
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
        Json(ApiResponse {
            ok: true,
            message: format!("Bem-vindo, {}!", db_username),
        }),
    )
        .into_response()
}

// -------------------- LOGOUT --------------------

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
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
        Json(ApiResponse {
            ok: true,
            message: "Logout realizado.".into(),
        }),
    )
        .into_response()
}

// -------------------- ME --------------------

async fn me(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let token = match get_session_token(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse {
                    ok: false,
                    message: "Não autenticado.".into(),
                }),
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
            Json(ApiResponse {
                ok: true,
                message: username,
            }),
        )
            .into_response(),
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse {
                ok: false,
                message: "Sessão inválida.".into(),
            }),
        )
            .into_response(),
    }
}

// -------------------- USERS --------------------

async fn list_users(State(state): State<AppState>) -> Response {
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
            Json(ApiResponse {
                ok: false,
                message: "Erro ao buscar usuários.".into(),
            }),
        )
            .into_response(),
    }
}

// -------------------- FORUM --------------------

async fn list_posts(State(state): State<AppState>) -> Response {
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

async fn create_post(
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

async fn get_post(State(state): State<AppState>, Path(id): Path<i32>) -> Response {
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

async fn add_comment(
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

// -------------------- UTILS --------------------

fn get_session_token(headers: &HeaderMap) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookies.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("session=") {
            return Some(val.to_string());
        }
    }
    None
}

// -------------------- STATIC --------------------

fn wrap_page(title: &str, body: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="pt-BR">
<head>
  <meta charset="UTF-8">
  <link rel="stylesheet" href="/assets/css/style.css">
  <title>{title}</title>
</head>
<body>

<div id="header"></div>

{body}

<div id="footer"></div>

<script>
  fetch('/components/header.html')
    .then(r => r.text())
    .then(html => {{
      document.getElementById('header').innerHTML = html;
      return fetch('/me');
    }})
    .then(r => r.json())
    .then(me => {{
      const menu = document.getElementById('user-menu');
      if (!menu) return;
      if (me.ok) {{
        menu.innerHTML =
          '<span>Bem vindo, ' + me.message + '</span><br>' +
          '<a href="#" id="btnLogout">Logout</a>';
        document.getElementById('btnLogout').addEventListener('click', function(e) {{
          e.preventDefault();
          fetch('/logout', {{ method: 'POST' }}).then(() => window.location.href = '/');
        }});
      }}
    }})
    .catch(function() {{}});

  fetch('/components/footer.html')
    .then(r => r.text())
    .then(data => {{ document.getElementById('footer').innerHTML = data; }});
</script>

</body>
</html>"##,
        title = title,
        body = body,
    )
}

async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    let path = if path.is_empty() {
        "index".to_string()
    } else {
        path
    };

    let file_path = format!("{}.html", path);

    match fs::read_to_string(&file_path) {
        Ok(content) => Html(wrap_page("Santuario Thinkpad", &content)).into_response(),
        Err(_) => Html(wrap_page("404", "<main><article><h1>404 Not Found</h1></article></main>")).into_response(),
    }
}

async fn index() -> impl IntoResponse {
    match fs::read_to_string("index.html") {
        Ok(content) => Html(content).into_response(),
        Err(_) => Html("<h1>index.html não encontrado</h1>".to_string()).into_response(),
    }
}

async fn register_page() -> impl IntoResponse {
    match fs::read_to_string("u/register.html") {
        Ok(content) => Html(wrap_page("Criar conta - Santuario Thinkpad", &content)),
        Err(_) => Html(wrap_page("Erro", "<main><article><h1>register.html não encontrado</h1></article></main>")),
    }
}

async fn login_page() -> impl IntoResponse {
    match fs::read_to_string("u/login.html") {
        Ok(content) => Html(wrap_page("Login - Santuario Thinkpad", &content)),
        Err(_) => Html(wrap_page("Erro", "<main><article><h1>login.html não encontrado</h1></article></main>")),
    }
}

async fn forum_page() -> impl IntoResponse {
    match fs::read_to_string("s.html") {
        Ok(content) => Html(wrap_page("Forum - Santuario Thinkpad", &content)),
        Err(_) => Html(wrap_page("Erro", "<main><article><h1>s.html não encontrado</h1></article></main>")),
    }
}

async fn post_page(Path(_id): Path<i32>) -> impl IntoResponse {
    match fs::read_to_string("s/post.html") {
        Ok(content) => Html(wrap_page("Post - Santuario Thinkpad", &content)),
        Err(_) => Html(wrap_page("Erro", "<main><article><h1>post.html não encontrado</h1></article></main>")),
    }
}
