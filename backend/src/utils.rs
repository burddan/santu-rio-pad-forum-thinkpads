use axum::http::{header, HeaderMap};

pub fn get_session_token(headers: &HeaderMap) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookies.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("session=") {
            return Some(val.to_string());
        }
    }
    None
}

pub fn wrap_page(title: &str, body: &str) -> String {
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
