# 🌐 garos-control-api

> **Serviço Backend de API REST e WebSockets em Rust (Axum / SQLx) para Gestão de Frota e Telemetria em Tempo Real.**

[![Rust](https://img.shields.io/badge/Rust-1.80+-orange.svg?logo=rust)](https://www.rust-lang.org)
[![Axum](https://img.shields.io/badge/Framework-Axum-black.svg)](https://github.com/tokio-rs/axum)
[![SQLx](https://img.shields.io/badge/Database-SQLite%20%2F%20Postgres-blue.svg?logo=sqlite)](https://github.com/launchbadge/sqlx)
[![License](https://img.shields.io/badge/License-Proprietary-red.svg)](#)

---

## 📌 Visão Geral

O **`garos-control-api`** é o serviço central de controle do GAROS, responsável por fornecer endpoints REST autenticados via JWT RS256 e canais WebSocket para a gestão centralizada do parque de estações diskless e do servidor `srv-garos`.

### Funcionalidades Principais
- 📡 **Heartbeat & Telemetria de Nós (`/api/garos/nodes`)**: Auto-registro de clientes diskless, telemetria de CPU/RAM/Disco e status de conectividade em tempo real.
- 🔐 **Autenticação & Autorização JWT (`/api/garos/auth`)**: Emissão e validação de tokens JWT RS256 com escopos de permissão baseados em papéis (RBAC).
- 💾 **Gestão de Storage & Imagens (`/api/garos/images`, `/storage`)**: Endpoints de controle para compilação, rolagem e auditoria de snapshots BTRFS.
- ⚡ **WebSockets em Tempo Real (`/api/garos/ws`)**: Streaming de eventos do sistema, logs em tempo real e notificações de alteração de estado dos clientes.
- 🛡️ **Auditoria & Firewall (`/api/garos/audit`, `/firewall`)**: Registro imutável de ações administrativas e manipulação de regras de rede.

---

## 🏗️ Arquitetura e Módulos Rust

```mermaid
graph TD
    Client[garos-control-web / gar CLI] -->|HTTP / REST JSON| Router[Axum Router :8000]
    Client -->|WebSocket Upgrade| WS[WebSocket Handler]

    subgraph Middleware Layer
        Auth[JWT RS256 Auth Middleware]
        Cors[CORS Allowlist]
        Tracing[OpenTelemetry Tracing]
    end

    subgraph API Handlers (/src/handlers)
        NodesH[nodes.rs - Heartbeat & Nodes]
        UsersH[users.rs - SSOT Users Sync]
        ImagesH[images.rs - Netboot Images]
        AuditH[audit.rs - Audit Trail]
        StorageH[storage.rs - BTRFS Snapshots]
    end

    subgraph Data & Integrations
        DB[(SQLite / PostgreSQL Database)]
        GarCLI[gar CLI Wrapper]
    end

    Router --> Auth
    Auth --> NodesH
    Auth --> UsersH
    Auth --> ImagesH
    Auth --> AuditH
    Auth --> StorageH
    NodesH --> DB
    UsersH --> DB
    ImagesH --> GarCLI
    WS --> DB
```

### 📂 Estrutura de Diretórios

```text
garos-control-api/
├── Cargo.toml               # Manifesto Rust com Axum, SQLx, Tokio, Tower-HTTP, JsonWebToken
├── Cargo.lock               # Trava de dependências Rust
├── Dockerfile               # Build multi-stage otimizado para produção
├── docker-compose.yml       # Stack Docker com API + PostgreSQL / SQLite
├── migrations/              # Migrações SQLx (criação de tabelas nodes, audit_logs, users)
├── RELATORIO_BACKEND.md     # Relatório de auditoria e status de compilação
├── INVENTORY.md             # Especificação dos endpoints e modelo de dados
├── src/
│   ├── main.rs              # Inicialização do Tokio runtime, pool SQLx e servidor HTTP
│   ├── config.rs            # Configurações de ambiente via dotenv/env
│   ├── state.rs             # Application State compartilhado (AppState)
│   ├── handlers/            # Controladores das rotas REST e WebSockets
│   │   ├── nodes.rs         # Gerenciamento de nós e heartbeat
│   │   ├── users.rs         # Sincronização de usuários
│   │   ├── images.rs        # Gerenciamento de imagens netboot
│   │   ├── storage.rs       # Gestão de armazenamento
│   │   ├── audit.rs         # Registro de auditoria
│   │   └── ws.rs            # WebSocket Hub e broadcast
│   ├── db/                  # Repositórios SQLx e consultas preparadas
│   ├── middleware/          # Middlewares de autenticação JWT e CORS
│   └── realtime/            # Gerenciador de sessões e canais WebSocket
└── tests/                   # Suíte de testes de integração e endpoints
```

---

## ⚡ Desenvolvimento e Execução Local

### 1. Configuração do Ambiente

Crie o arquivo `.env` baseado no `.env.example`:

```bash
DATABASE_URL="sqlite://garos_control.db?mode=rwc"
PORT=8000
JWT_SECRET="sua-chave-secreta-para-desenvolvimento"
LOG_LEVEL="info"
```

### 2. Rodar a Aplicação

```bash
# Executar as migrações do banco de dados
cargo sqlx migrate run

# Subir a API em modo de desenvolvimento com hot-reload
cargo run

# Executar a suíte de testes unitários e de integração
cargo test
```

### 3. Execução via Docker Compose

```bash
docker-compose up --build -d
```

---

## 📄 Relatórios & Documentação Interna

- 📋 [`INVENTORY.md`](./INVENTORY.md) — Inventário completo dos esquemas e endpoints.
- 📊 [`RELATORIO_BACKEND.md`](./RELATORIO_BACKEND.md) — Relatório de implementação e débito técnico em resolução.

---

## 📄 Veja Também

- 📘 [GAROS Core Repository](file:///home/garton/Projetos/garos-dev/GAROS/README.md)
- 📊 [GAROS Control Web Dashboard](file:///home/garton/Projetos/garos-dev/garos-control-web/README.md)
- 🧠 [Decisões Arquiteturais no Vault](file:///home/garton/Projetos/garos-dev/garos-think-vault/README.md)
