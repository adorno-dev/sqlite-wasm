# sqlite-wasm

[![crates.io](https://img.shields.io/crates/v/sqlite-wasm.svg)](https://crates.io/crates/sqlite-wasm)
[![docs.rs](https://docs.rs/sqlite-wasm/badge.svg)](https://docs.rs/sqlite-wasm)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Wrapper Rust** para o [SQLite-WASM oficial](https://sqlite.org/wasm), permitindo uso idiomático em projetos Rust com acesso ao OPFS (Origin Private File System).

Este crate não reinventa a roda - ele empacota e expõe as funcionalidades do SQLite compilado para WASM de forma simples e ergonômica para Rust.

## ✨ Características

- 🦀 **API Rust idiomática** - Use `async`/`await` naturalmente
- 🗄️ **Core oficial** - Baseado no SQLite-WASM do [sqlite.org](https://sqlite.org/wasm)
- 💾 **Persistência real** - OPFS (Origin Private File System) para dados persistentes
- 🔄 **Wrapper leve** - Apenas o necessário para integrar Rust com o SQLite-WASM
- 📦 **Auto-suficiente** - Inclui os arquivos oficiais do SQLite-WASM

## 📦 Instalação

Adicione ao seu `Cargo.toml`:

```toml
[dependencies]
sqlite-wasm = { git = "https://github.com/adorno-dev/sqlite-wasm.git", branch = "development" }
```

## 🚀 Uso Básico

```rust
use sqlite_wasm::{exec, query};
use wasm_bindgen_futures::spawn_local;
use js_sys::Array;

async fn exemplo() -> Result<(), JsValue> {
    // Criar tabela
    exec(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
        )".to_string(),
        Array::new()
    ).await?;

    // Inserir dados
    let bind = Array::new();
    bind.push(&"João".into());
    exec("INSERT INTO users (name) VALUES (?)".to_string(), bind).await?;

    // Consultar
    let result = query("SELECT * FROM users".to_string(), None).await?;
    println!("Resultado: {:?}", result);
    
    Ok(())
}
```

## 🔧 Arquivos Oficiais do SQLite

Este crate inclui e gerencia os arquivos oficiais do SQLite-WASM:
- `sqlite3.js` - Core do SQLite
- `sqlite3-worker1.js` - Worker para operações assíncronas
- `sqlite3-opfs-async-proxy.js` - Proxy para OPFS
- `sqlite3.wasm` - Módulo WebAssembly

### Copiando os arquivos para seu projeto

Após compilar, copie os arquivos oficiais para sua pasta pública:

```bash
cp -r $(find target -path "*/build/sqlite-wasm-*/out/jswasm" | head -1) ./public
```

### Headers HTTP obrigatórios

Seu servidor precisa enviar estes headers para o OPFS funcionar:

```javascript
// Exemplo com Express
app.use((req, res, next) => {
  res.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
  res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
  next();
});
```

## 📖 API

### `async fn exec(sql: String, bind: Array) -> Result<(), JsValue>`
Executa comandos SQL sem retorno (INSERT, UPDATE, DELETE, CREATE).  
- `sql`: Comando SQL
- `bind`: Parâmetros para placeholders `?`

### `async fn query(sql: String, bind: Option<Array>) -> Result<JsValue, JsValue>`
Executa uma consulta SQL e retorna as linhas como array de objetos.  
- `sql`: Consulta SQL
- `bind`: Parâmetros opcionais

### `async fn init_db() -> Result<JsValue, JsValue>`
Inicializa o banco de dados e cria a tabela padrão `users`. Retorna o `dbId`.

### `fn get_worker() -> Worker`
Retorna o worker do SQLite (para uso avançado).

## 🏗️ Exemplo Completo com Leptos

```rust
use leptos::*;
use sqlite_wasm::{exec, query};
use wasm_bindgen_futures::spawn_local;
use js_sys::Array;

#[component]
fn App() -> impl IntoView {
    let (users, set_users) = create_signal(Vec::new());
    let (status, set_status) = create_signal("Carregando...".to_string());

    create_effect(move |_| {
        spawn_local(async move {
            // Inicializa banco
            match sqlite_wasm::init_db().await {
                Ok(_) => {
                    set_status.set("Banco pronto!".to_string());
                    
                    // Carrega usuários
                    let result = query("SELECT * FROM users".to_string(), None).await.unwrap();
                    // Processa resultado...
                }
                Err(e) => set_status.set(format!("Erro: {:?}", e))
            }
        });
    });

    view! {
        <div>
            <h1>SQLite + Leptos</h1>
            <p>{status}</p>
        </div>
    }
}
```

## 🧪 Testando Localmente

1. Clone o repositório:
```bash
git clone https://github.com/adorno-dev/sqlite-wasm.git
cd sqlite-wasm
```

2. Compile para WASM:
```bash
wasm-pack build --target web
```

3. Use o servidor de teste incluso:
```bash
npm install express
node server.js
```

## 🏗️ Arquitetura

```
Seu Código Rust → sqlite-wasm (wrapper) → SQLite-WASM Oficial → OPFS (navegador)
```

## 🙏 Créditos

- [SQLite](https://sqlite.org) - Banco de dados incrível
- [SQLite-WASM](https://sqlite.org/wasm) - Versão oficial para WebAssembly
- [Rust](https://rust-lang.org) - Linguagem maravilhosa
- [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) - Integração Rust-WASM

## 📄 Licença

MIT - assim como o SQLite é domínio público, este wrapper é livre.

## 🤝 Contribuindo

Contribuições são bem-vindas! Por favor:

1. Fork o projeto
2. Crie uma branch (`git checkout -b feature/AmazingFeature`)
3. Commit suas mudanças (`git commit -m 'Add AmazingFeature'`)
4. Push (`git push origin feature/AmazingFeature`)
5. Abra um Pull Request

## ⚠️ Limitações Conhecidas

- Requer navegadores modernos com suporte a WASM e OPFS
- Headers COOP/COEP obrigatórios no servidor
- Funciona apenas em contexto seguro (localhost ou HTTPS)
- Dados ficam restritos à origem (site) do navegador

## 📞 Suporte

- Issues: [GitHub Issues](https://github.com/adorno-dev/sqlite-wasm/issues)
- Discussões: [GitHub Discussions](https://github.com/adorno-dev/sqlite-wasm/discussions)

---

**Feito com ❤️ para a comunidade Rust + WASM**

