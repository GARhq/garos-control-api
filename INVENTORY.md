# INVENTORY — garos-control-api

> **Auto-gerado via script Aura em 2026-08-13.** Surface of truth da API.
> Source: `~/Proyectos/garos-dev/garos-control-api/`

## Visão geral

| Métrica | Valor |
|---|---|
| Linguagem | Rust (edition 2021, MSRV 1.82) |
| LOC total (`src/`) | 10.899 |
| Arquivos `.rs` | ~50 |
| Dependências (`Cargo.toml`) | 90 |
| Migrations SQL | 1 (`20240101000001_initial.sql`, 8.3 KB) |
| Entry bin | `garos-backend` |
| Lib | `garos_backend` |
| Features | `sqlite` (default), `postgres` |

## Stack

- **Web framework:** axum 0.7 (com `macros`, `ws`, `http2`)
- **Tower:** `limit`, `load-shed`, `timeout`
- **tower-http:** `trace`, `cors`, `compression-{gzip,br}`, `request-id`, `util`, `set-header`
- **DB:** sqlx (sqlite + postgres)
- **Auth:** jsonwebtoken, ldap3
- **Async runtime:** tokio
- **Serialização:** serde, serde_json
- **Errors:** thiserror, anyhow

## Estrutura de pastas

```
src/
├── main.rs                        # Entry point (260 linhas)
├── lib.rs                         # Library root
├── state.rs                       # AppState compartilhado
├── api/
│   ├── mod.rs                     # Router principal (~50 rotas)
│   ├── error.rs                   # ApiError tipo
│   └── openapi.rs                 # Spec OpenAPI/Swagger
├── auth/
│   ├── mod.rs
│   └── jwt.rs                     # JWT validation (264 linhas)
├── db/
│   ├── mod.rs, pool.rs
│   ├── models/                    # 7+ models
│   │   ├── mod.rs
│   │   ├── audit_log.rs
│   │   ├── firewall_rule.rs
│   │   ├── image.rs
│   │   ├── node.rs
│   │   ├── service.rs
│   │   ├── storage_snapshot.rs
│   │   └── user.rs
│   └── repositories/              # 8+ repos
│       ├── mod.rs
│       ├── audit.rs, firewall.rs, images.rs
│       ├── nodes.rs, services.rs, storage.rs
│       └── users.rs
├── integrations/                  # 10+ integrations Linux
│   ├── mod.rs
│   ├── btrfs.rs                   # BTRFS operations
│   ├── journald.rs                # journald log reader
│   ├── nftables.rs                # Firewall rules
│   ├── nix.rs                     # Nix integration
│   ├── pxe.rs                     # PXE/iPXE
│   ├── samba.rs                   # Samba/NFS
│   ├── systemd.rs                 # systemd services
│   └── wol.rs                     # Wake-on-LAN
└── migrations/
    └── 20240101000001_initial.sql # Schema inicial
```

## Endpoints REST (~50 rotas)

### Health/observability
- `GET /health` — Liveness probe
- `GET /ready` — Readiness probe
- `GET /metrics` — Prometheus metrics
- `GET /version` — Versão do binário
- `GET /docs` — Swagger UI
- `GET /api-docs/openapi.json` — Spec OpenAPI

### Autenticação (`/api/auth/*`)
- `POST /api/auth/login` — Login (LDAP-backed)
- `POST /api/auth/logout` — Logout (revoga token)
- `GET /api/auth/me` — User atual
- `POST /api/auth/refresh` — Refresh JWT

### Nodes (`/api/garos/nodes/*`)
- `GET /api/garos/nodes` — Lista nodes
- `POST /api/garos/nodes/bulk/reimage` — Reimagem N nodes (DESTRUTIVO)
- `POST /api/garos/nodes/bulk/shutdown` — Shutdown N nodes (DESTRUTIVO)
- `POST /api/garos/nodes/bulk/wol` — Wake N nodes
- `GET /api/garos/nodes/{mac}` — Detalhe node
- `POST /api/garos/nodes/{mac}/heartbeat` — Heartbeat
- `POST /api/garos/nodes/{mac}/maintenance` — Toggle maintenance mode
- `POST /api/garos/nodes/{mac}/reboot` — Reboot (DESTRUTIVO)
- `POST /api/garos/nodes/{mac}/reimage` — Reimagem (DESTRUTIVO)
- `POST /api/garos/nodes/{mac}/shutdown` — Shutdown (DESTRUTIVO)
- `GET /api/garos/nodes/{mac}/events` — Eventos do node

### Images (`/api/garos/images/*`)
- `GET /api/garos/images` — Lista imagens publicadas
- `POST /api/garos/images/{id}/build` — Build nova versão
- `GET /api/garos/images/{id}/build/status` — Status do build
- `GET /api/garos/images/{id}/diff/{versionA}/{versionB}` — Diff entre versões
- `POST /api/garos/images/{id}/publish` — Publicar versão (DESTRUTIVO)
- `POST /api/garos/images/{id}/unpublish` — Despublicar (DESTRUTIVO)
- `GET /api/garos/images/{id}/versions` — Versões publicadas
- `GET /api/garos/images/{id}/stations` — Estações usando esta imagem

### Firewall (`/api/garos/firewall/*`)
- `GET /api/garos/firewall/rules` — Lista rules
- `POST /api/garos/firewall/rules` — Cria rule
- `GET /api/garos/firewall/rules/preview` — Preview de rule antes de aplicar
- `GET /api/garos/firewall/rules/{id}` — Detalhe rule
- `POST /api/garos/firewall/panic` — MODO PANICO (DESTRUTIVO TOTAL)
- `GET /api/garos/firewall/panic/status` — Status do panic mode
- `GET /api/garos/firewall/connections` — Conexões ativas
- `POST /api/garos/firewall/validate` — Valida regras

### Metrics / Audit / Activity
- `GET /api/garos/metrics` — Métricas agregadas
- `GET /api/garos/metrics/series` — Série temporal
- `GET /api/garos/metrics/sla` — SLA report
- `GET /api/garos/audit` — Audit log
- `GET /api/garos/audit/stats` — Stats do audit
- `GET /api/garos/audit/{id}` — Detalhe audit entry
- `POST /api/garos/audit/export` — Export audit (CSV/JSON)
- `GET /api/garos/activity` — Activity feed

## Como rodar (desenvolvimento)

```bash
cd ~/Proyectos/garos-dev/garos-control-api

# Deps
cargo build --release

# Migrations (requer sqlx-cli)
sqlx migrate run

# Run (SQLite local)
DB_URL=sqlite://./data/api.db cargo run --bin garos-backend --release
```

Default bind: `127.0.0.1:8081` (configurável via env).

## Como buildar (produção)

```bash
cd ~/Proyectos/garos-dev/garos-control-api
cargo build --release --bin garos-backend
# Output: target/release/garos-backend
```

Pra deploy via NixOS flake, ver K-004 (módulo `garos-control-api.nix` em `garos-installer/`).

## Segurança (cards AURA relacionados)

- AURA-20260813-002: API expõe ~50 rotas; validar rate limiting em endpoints destrutivos
- AURA-20260813-001: dependabot updates (postcss no installer — não afeta API diretamente)

## Como regenerar este INVENTORY

Script Python (em desenvolvimento na K-007) re-varre `src/` e atualiza este arquivo automaticamente.
