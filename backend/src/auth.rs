use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;

use crate::{
    models::{EntradaLogin, EntradaRegistro, Resposta},
    utils::pegar_token,
    AppState,
};

// Cria uma nova conta de usuário
pub async fn registrar(
    State(estado): State<AppState>,
    Json(dados): Json<EntradaRegistro>,
) -> Response {
    let usuario = dados.usuario.trim().to_string();
    let senha   = dados.senha.trim().to_string();

    if usuario.is_empty() || senha.is_empty() {
        return erro(StatusCode::BAD_REQUEST, "Usuário e senha são obrigatórios.");
    }
    if senha.len() < 6 {
        return erro(StatusCode::BAD_REQUEST, "A senha deve ter pelo menos 6 caracteres.");
    }

    let hash = bcrypt::hash(&senha, bcrypt::DEFAULT_COST)
        .expect("falha ao gerar hash da senha");

    let resultado = sqlx::query("INSERT INTO usuarios (usuario, senha) VALUES ($1, $2)")
        .bind(&usuario)
        .bind(&hash)
        .execute(&estado.db)
        .await;

    match resultado {
        Ok(_) =>
            ok("Conta criada com sucesso!"),
        Err(e) if e.to_string().contains("duplicate key") =>
            erro(StatusCode::CONFLICT, "Nome de usuário já existe."),
        Err(_) =>
            erro(StatusCode::INTERNAL_SERVER_ERROR, "Erro interno ao registrar."),
    }
}

// Autentica o usuário e cria uma sessão com cookie
pub async fn entrar(
    State(estado): State<AppState>,
    Json(dados): Json<EntradaLogin>,
) -> Response {
    let usuario = dados.usuario.trim().to_string();
    let senha   = dados.senha.trim().to_string();

    // Busca o usuário no banco
    let linha = sqlx::query_as::<_, (i32, String, String)>(
        "SELECT id, usuario, senha FROM usuarios WHERE usuario = $1",
    )
    .bind(&usuario)
    .fetch_optional(&estado.db)
    .await;

    let (id_usuario, usuario_db, hash) = match linha {
        Ok(Some(r)) => r,
        Ok(None)    => return erro(StatusCode::UNAUTHORIZED, "Usuário ou senha incorretos."),
        Err(_)      => return erro(StatusCode::INTERNAL_SERVER_ERROR, "Erro interno."),
    };

    // Verifica a senha
    if !bcrypt::verify(&senha, &hash).unwrap_or(false) {
        return erro(StatusCode::UNAUTHORIZED, "Usuário ou senha incorretos.");
    }

    // Cria o token de sessão
    let token = Uuid::new_v4().to_string();
    if let Err(e) = sqlx::query(
        "INSERT INTO sessoes (token, id_usuario, usuario) VALUES ($1, $2, $3)",
    )
    .bind(&token)
    .bind(id_usuario)
    .bind(&usuario_db)
    .execute(&estado.db)
    .await
    {
        eprintln!("erro ao criar sessão: {:?}", e);
        return erro(StatusCode::INTERNAL_SERVER_ERROR, "Erro ao criar sessão.");
    }

    // Define o cookie de sessão (7 dias)
    let cookie = format!("session={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=604800", token);
    let mut cabecalhos = HeaderMap::new();
    cabecalhos.insert(header::SET_COOKIE, cookie.parse().unwrap());

    (
        StatusCode::OK,
        cabecalhos,
        Json(Resposta { ok: true, mensagem: format!("Bem-vindo, {}!", usuario_db) }),
    )
        .into_response()
}

// Encerra a sessão do usuário
pub async fn sair(State(estado): State<AppState>, cabecalhos: HeaderMap) -> Response {
    if let Some(token) = pegar_token(&cabecalhos) {
        sqlx::query("DELETE FROM sessoes WHERE token = $1")
            .bind(&token)
            .execute(&estado.db)
            .await
            .ok();
    }

    let limpar_cookie = "session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0";
    let mut h = HeaderMap::new();
    h.insert(header::SET_COOKIE, limpar_cookie.parse().unwrap());

    (StatusCode::OK, h, Json(Resposta { ok: true, mensagem: "Logout realizado.".into() }))
        .into_response()
}

// Retorna o usuário logado com base no cookie de sessão
pub async fn quem_sou(State(estado): State<AppState>, cabecalhos: HeaderMap) -> Response {
    let token = match pegar_token(&cabecalhos) {
        Some(t) => t,
        None    => return erro(StatusCode::UNAUTHORIZED, "Não autenticado."),
    };

    let linha = sqlx::query_as::<_, (String,)>(
        "SELECT usuario FROM sessoes WHERE token = $1",
    )
    .bind(&token)
    .fetch_optional(&estado.db)
    .await;

    match linha {
        Ok(Some((usuario,))) =>
            (StatusCode::OK, Json(Resposta { ok: true, mensagem: usuario })).into_response(),
        _ =>
            erro(StatusCode::UNAUTHORIZED, "Sessão inválida."),
    }
}

// Auxiliares para respostas padronizadas
fn ok(msg: &str) -> Response {
    (StatusCode::OK, Json(Resposta { ok: true, mensagem: msg.into() })).into_response()
}

fn erro(status: StatusCode, msg: &str) -> Response {
    (status, Json(Resposta { ok: false, mensagem: msg.into() })).into_response()
}
