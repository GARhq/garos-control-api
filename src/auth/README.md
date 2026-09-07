# auth

## 📌 Visão Geral
Módulo integrante da arquitetura do repositório GAROS no caminho `garos-control-api/src/auth`.

## ⚙️ O que esta pasta faz
- Fornecer a lógica executável de backend, ferramentas CLI ou componentes de alta performance em Rust.

## 📂 Conteúdo e Arquivos Detalhados
- `extractor.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `CurrentUser` e funções `from_claims, from_request_parts`.
- `jwt.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `Claims, TokenPair, AuthUser` e enums `Role` e funções `as_str, from_str, default_kind`.
- `middleware.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo funções `extract_token, extract_token_or_query, require_auth`.
- `mod.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust com lógica de execução de sistema em Rust.
- `password.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo funções `hash_password, verify_password, hex`.

## 🔄 Histórico de Mudanças Comportamentais
- **[Inicial]**: Mapeamento e documentação detalhada da estrutura inicial do módulo.
