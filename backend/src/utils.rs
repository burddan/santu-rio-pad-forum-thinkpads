use axum::http::{header, HeaderMap};

// Extrai o token de sessão do cookie da requisição
pub fn pegar_token(headers: &HeaderMap) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    for parte in cookies.split(';') {
        let parte = parte.trim();
        if let Some(token) = parte.strip_prefix("session=") {
            return Some(token.to_string());
        }
    }
    None
}

// Envolve o conteúdo HTML parcial em um documento completo com CSS, header e footer
pub fn montar_pagina(titulo: &str, corpo: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="pt-BR">
<head>
  <meta charset="UTF-8">
  <link rel="stylesheet" href="/assets/css/style.css">
  <title>{titulo}</title>
</head>
<body>

<div id="header"></div>

{corpo}

<div id="footer"></div>

<script>
  // Carrega o header e verifica se o usuário está logado
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
          '<span>Bem vindo, ' + me.mensagem + '</span><br>' +
          '<a href="#" id="btnLogout">Logout</a>';
        document.getElementById('btnLogout').addEventListener('click', function(e) {{
          e.preventDefault();
          fetch('/logout', {{ method: 'POST' }}).then(() => window.location.href = '/');
        }});
      }}
    }})
    .catch(function() {{}});

  // Carrega o footer
  fetch('/components/footer.html')
    .then(r => r.text())
    .then(html => {{ document.getElementById('footer').innerHTML = html; }});
</script>

</body>
</html>"##,
        titulo = titulo,
        corpo = corpo,
    )
}
