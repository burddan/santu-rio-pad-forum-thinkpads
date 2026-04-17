use serde::{Deserialize, Serialize};

// --- Entrada de dados ---

#[derive(Deserialize)]
pub struct EntradaRegistro {
    pub usuario: String,
    pub senha: String,
}

#[derive(Deserialize)]
pub struct EntradaLogin {
    pub usuario: String,
    pub senha: String,
}

#[derive(Deserialize)]
pub struct EntradaComentario {
    pub conteudo: String,
}

// --- Respostas da API ---

#[derive(Serialize)]
pub struct Resposta {
    pub ok: bool,
    pub mensagem: String,
}

// --- Dados de usuário ---

#[derive(Serialize)]
pub struct UsuarioItem {
    pub usuario: String,
}

// --- Dados do fórum ---

#[derive(Serialize)]
pub struct PostResumo {
    pub id: i32,
    pub usuario: String,
    pub titulo: String,
    pub conteudo: String,
    pub imagem: Option<String>,
    pub criado_em: String,
    pub total_comentarios: i64,
}

#[derive(Serialize)]
pub struct ComentarioItem {
    pub id: i32,
    pub usuario: String,
    pub conteudo: String,
    pub criado_em: String,
}

#[derive(Serialize)]
pub struct PostCompleto {
    pub id: i32,
    pub usuario: String,
    pub titulo: String,
    pub conteudo: String,
    pub imagem: Option<String>,
    pub criado_em: String,
    pub comentarios: Vec<ComentarioItem>,
}
