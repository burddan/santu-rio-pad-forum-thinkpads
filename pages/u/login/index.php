<?php include '../../header.php'; ?>
<main>

<article>
  <h1>Login</h1>

    <fieldset>
      <label for="username">Usuário</label>
      <input 
        type="text" 
        id="username" 
        name="username" 
        tabindex="1" 
        required 
        autofocus
      >
    </fieldset>

    <fieldset>
      <label for="password">Senha </label>
      <input 
        type="password" 
        id="password" 
        name="password" 
        tabindex="2" 
        required 
        placeholder="Sua senha"
      >
    </fieldset>

    <fieldset class="options">
      <a href="esqueci-senha.php" tabindex="4" class="forgot">Esqueci minha senha</a>
    </fieldset>
<fieldset>
  <p>
  Ainda não tem conta? <a href="/u/register/index.php">Criar uma conta</a>
  </p>
</fieldset>

<fieldset>
<button 
      type="submit" 
      tabindex="5"
      style="width: 100px; height: 30; font-size: 18px; padding: 10px;"
    >
      Entrar
    </button>

</fieldset>

</article>
  <?php include '../../footer.php'; ?>

</main>
</body>
</html>

