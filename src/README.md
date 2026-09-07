# src

## 📌 Visão Geral
Código-fonte da API REST, handlers de rota, repositórios de banco de dados e WebSockets.

## ⚙️ O que esta pasta faz
- Fornecer a lógica executável de backend, ferramentas CLI ou componentes de alta performance em Rust.

## 📂 Conteúdo e Arquivos Detalhados
- `api/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `api`.
- `auth/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `auth`.
- `db/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `db`.
- `domain/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `domain`.
- `handlers/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `handlers`.
- `integrations/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `integrations`.
- `middleware/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `middleware`.
- `realtime/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `realtime`.
- `services/`: **[Módulo]** — Subsistema e arquivos de organização do módulo `services`.
- `config.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `Settings, ServerSettings, DatabaseSettings` e funções `socket_addr, request_timeout, default`.
- `error.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `FieldError` e enums `IntegrationKind, AppError` e funções `as_str, status, code`.
- `lib.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo enums `and`.
- `main.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `Cli` e enums `Cmd, UserCmd` e funções `main, to_anyhow, serve`.
- `metrics.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `Metrics` e funções `new, render`.
- `state.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `AppState` e funções `fmt, new`.
- `telemetry.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `TelemetryGuard` e funções `drop, init, make_request_span`.

## 🔄 Histórico de Mudanças Comportamentais
- **[Inicial]**: Mapeamento e documentação detalhada da estrutura inicial do módulo.
