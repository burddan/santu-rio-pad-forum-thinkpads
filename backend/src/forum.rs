use axum::{
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;

use crate::{
    models::{ComentarioItem, EntradaComentario, PostCompleto, PostResumo, Resposta},
    utils::pegar_token,
    AppState,
};

// Lista todos os posts com contagem de comentários, do mais recente ao mais antigo
pub async fn listar_posts(State(estado): State<AppState>) -> Response {
    let linhas = sqlx::query_as::<_, (i32, String, String, String, Option<String>, String, i64)>(
        r#"SELECT p.id, p.usuario, p.titulo, p.conteudo, p.imagem,
                  TO_CHAR(p.criado_em AT TIME ZONE 'America/Sao_Paulo', 'DD/MM/YYYY HH24:MI') AS criado_em,
                  COUNT(c.id) AS total_comentarios
           FROM posts p
           LEFT JOIN comentarios c ON c.post_id = p.id
           GROUP BY p.id
           ORDER BY p.criado_em DESC"#,
    )
    .fetch_all(&estado.db)
    .await;

    match linhas {
        Ok(linhas) => {
            let posts: Vec<PostResumo> = linhas
                .into_iter()
                .map(|(id, usuario, titulo, conteudo, imagem, criado_em, total_comentarios)| {
                    PostResumo { id, usuario, titulo, conteudo, imagem, criado_em, total_comentarios }
                })
                .collect();
            (StatusCode::OK, Json(posts)).into_response()
        }
        Err(e) => {
            eprintln!("erro ao listar posts: {:?}", e);
            erro(StatusCode::INTERNAL_SERVER_ERROR, "Erro ao buscar posts.")
        }
    }
}

// Cria um novo post (requer sessão ativa). Aceita título, conteúdo e imagem opcional
pub async fn criar_post(
    State(estado): State<AppState>,
    cabecalhos: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    let usuario = match autenticar(&cabecalhos, &estado).await {
        Some(u) => u,
        None    => return erro(StatusCode::UNAUTHORIZED, "Não autenticado."),
    };

    // Lê os campos do formulário multipart
    let mut titulo   = String::new();
    let mut conteudo = String::new();
    let mut imagem: Option<String> = None;

    while let Ok(Some(campo)) = multipart.next_field().await {
        match campo.name().unwrap_or("") {
            "titulo"   => titulo   = campo.text().await.unwrap_or_default(),
            "conteudo" => conteudo = campo.text().await.unwrap_or_default(),
            "imagem"   => {
                let extensao = campo.file_name()
                    .and_then(|f| f.rsplit('.').next())
                    .unwrap_or("jpg")
                    .to_string();
                let dados = campo.bytes().await.unwrap_or_default();
                if !dados.is_empty() {
                    let nome_arquivo = format!("{}.{}", Uuid::new_v4(), extensao);
                    let caminho = format!("uploads/{}", nome_arquivo);
                    if tokio::fs::write(&caminho, &dados).await.is_ok() {
                        imagem = Some(format!("/uploads/{}", nome_arquivo));
                    }
                }
            }
            _ => {}
        }
    }

    let titulo   = titulo.trim().to_string();
    let conteudo = conteudo.trim().to_string();

    if titulo.is_empty() || conteudo.is_empty() {
        return erro(StatusCode::BAD_REQUEST, "Título e conteúdo são obrigatórios.");
    }

    let resultado = sqlx::query(
        "INSERT INTO posts (usuario, titulo, conteudo, imagem) VALUES ($1, $2, $3, $4)",
    )
    .bind(&usuario)
    .bind(&titulo)
    .bind(&conteudo)
    .bind(&imagem)
    .execute(&estado.db)
    .await;

    match resultado {
        Ok(_)  => ok("Post criado!"),
        Err(e) => {
            eprintln!("erro ao criar post: {:?}", e);
            erro(StatusCode::INTERNAL_SERVER_ERROR, "Erro ao criar post.")
        }
    }
}

// Retorna um post completo com todos os comentários
pub async fn pegar_post(State(estado): State<AppState>, Path(id): Path<i32>) -> Response {
    let post = sqlx::query_as::<_, (i32, String, String, String, Option<String>, String)>(
        r#"SELECT id, usuario, titulo, conteudo, imagem,
                  TO_CHAR(criado_em AT TIME ZONE 'America/Sao_Paulo', 'DD/MM/YYYY HH24:MI') AS criado_em
           FROM posts WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&estado.db)
    .await;

    let (id_post, usuario, titulo, conteudo, imagem, criado_em) = match post {
        Ok(Some(r)) => r,
        Ok(None)    => return erro(StatusCode::NOT_FOUND, "Post não encontrado."),
        Err(_)      => return erro(StatusCode::INTERNAL_SERVER_ERROR, "Erro ao buscar post."),
    };

    let comentarios = sqlx::query_as::<_, (i32, String, String, String)>(
        r#"SELECT id, usuario, conteudo,
                  TO_CHAR(criado_em AT TIME ZONE 'America/Sao_Paulo', 'DD/MM/YYYY HH24:MI') AS criado_em
           FROM comentarios WHERE post_id = $1 ORDER BY criado_em ASC"#,
    )
    .bind(id)
    .fetch_all(&estado.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(id, usuario, conteudo, criado_em)| ComentarioItem { id, usuario, conteudo, criado_em })
    .collect();

    (StatusCode::OK, Json(PostCompleto { id: id_post, usuario, titulo, conteudo, imagem, criado_em, comentarios }))
        .into_response()
}

// Adiciona um comentário em um post (requer sessão ativa)
pub async fn comentar(
    State(estado): State<AppState>,
    cabecalhos: HeaderMap,
    Path(id_post): Path<i32>,
    Json(dados): Json<EntradaComentario>,
) -> Response {
    let usuario = match autenticar(&cabecalhos, &estado).await {
        Some(u) => u,
        None    => return erro(StatusCode::UNAUTHORIZED, "Não autenticado."),
    };

    let conteudo = dados.conteudo.trim().to_string();
    if conteudo.is_empty() {
        return erro(StatusCode::BAD_REQUEST, "Comentário vazio.");
    }

    let resultado = sqlx::query(
        "INSERT INTO comentarios (post_id, usuario, conteudo) VALUES ($1, $2, $3)",
    )
    .bind(id_post)
    .bind(&usuario)
    .bind(&conteudo)
    .execute(&estado.db)
    .await;

    match resultado {
        Ok(_)  => ok("Comentário adicionado."),
        Err(_) => erro(StatusCode::INTERNAL_SERVER_ERROR, "Erro ao comentar."),
    }
}

// Verifica a sessão e retorna o nome do usuário logado
async fn autenticar(cabecalhos: &HeaderMap, estado: &AppState) -> Option<String> {
    let token = pegar_token(cabecalhos)?;
    sqlx::query_as::<_, (String,)>("SELECT usuario FROM sessoes WHERE token = $1")
        .bind(&token)
        .fetch_optional(&estado.db)
        .await
        .ok()
        .flatten()
        .map(|(u,)| u)
}

fn ok(msg: &str) -> Response {
    (StatusCode::OK, Json(Resposta { ok: true, mensagem: msg.into() })).into_response()
}

fn erro(status: StatusCode, msg: &str) -> Response {
    (status, Json(Resposta { ok: false, mensagem: msg.into() })).into_response()
}
