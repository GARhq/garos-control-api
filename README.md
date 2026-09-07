# garos-control-api

## 📌 Visão Geral
Serviço backend de API REST em Rust (Axum/SQLx) para gerenciamento de frota e comunicação em tempo real.

## ⚙️ O que esta pasta faz
- Gerenciar especificações declarativas do NixOS, módulos do sistema e receitas de build.
- Manter documentadas as decisões de design, especificações de rotas e guias operacionais.

## 📂 Conteúdo e Arquivos Detalhados
- `config/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `config`.
- `examples/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `examples`.
- `migrations/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `migrations`.
- `scripts/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `scripts`.
- `src/`: **[Módulo]** — Código-fonte da API REST, handlers de rota, repositórios de banco de dados e WebSockets.
- `tests/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `tests`.
- `Cargo.lock`: **[Trava de Versões Cargo]** — Arquivo de trava determinístico contendo a árvore completa de crates e versões compiladas em Rust.
- `Cargo.toml`: **[Manifesto Cargo Rust]** — Especifica metadados do pacote Rust, dependências (rust-version, description, license, resolver) e perfis de compilação.
- `Dockerfile`: **[Arquivo do Módulo]** — Arquivo integrante do diretório (Dockerfile).
- `INVENTORY.md`: **[Documentação em Markdown]** — Documento de especificação técnica abordando *INVENTORY — garos-control-api*.
- `RELATORIO_BACKEND.md`: **[Documentação em Markdown]** — Documento de especificação técnica abordando *Relatório de Implementação — `garos-backend`*.
- `docker-compose.dev.yml`: **[Arquivo do Módulo]** — Arquivo integrante do diretório (docker-compose.dev.yml).
- `docker-compose.yml`: **[Arquivo do Módulo]** — Arquivo integrante do diretório (docker-compose.yml).
- `flake.lock`: **[Trava de Dependências Flake]** — Registro determinístico com hashes e revisões exatas dos repositórios e módulos importados pelo Flake.
- `flake.nix`: **[Nix Flake Principal]** — Define as entradas (nixpkgs, flake-utils) e configurações de sistema () do ecossistema Nix.

## 🔄 Histórico de Mudanças Comportamentais
- **[Inicial]**: Mapeamento e documentação detalhada da estrutura inicial do módulo.
