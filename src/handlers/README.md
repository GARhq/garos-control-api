# handlers

## 📌 Visão Geral
Módulo integrante da arquitetura do repositório GAROS no caminho `garos-control-api/src/handlers`.

## ⚙️ O que esta pasta faz
- Fornecer a lógica executável de backend, ferramentas CLI ou componentes de alta performance em Rust.

## 📂 Conteúdo e Arquivos Detalhados
- `activity.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `ActivityQuery` e funções `feed`.
- `audit.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `AuditListQuery, ExportQuery` e funções `parse_dt, list, by_id`.
- `auth.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `MeResponse` e funções `login, refresh, logout`.
- `firewall.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `ConnectionsQuery` e funções `list, create, by_id`.
- `health.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `VersionInfo` e funções `health, ready, metrics`.
- `images.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo funções `image_view, list, create`.
- `metrics.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `SeriesQuery` e funções `snapshot, series, sla`.
- `mod.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust com lógica de execução de sistema em Rust.
- `nodes.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `ListQuery` e funções `default_limit, default_sort, default_order`.
- `services.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `LogsQuery` e funções `list, by_name, start`.
- `storage.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `PoolNamePath` e funções `pools, pool_usage, start_scrub`.
- `users.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `ListUsersQuery` e funções `user_view, list, create`.
- `ws.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `WsQuery` e enums `WsClientMessage, WsServerMessage` e funções `ws_handler, client_loop`.

## 🔄 Histórico de Mudanças Comportamentais
- **[Inicial]**: Mapeamento e documentação detalhada da estrutura inicial do módulo.
