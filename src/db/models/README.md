# models

## 📌 Visão Geral
Módulo integrante da arquitetura do repositório GAROS no caminho `garos-control-api/src/db/models`.

## ⚙️ O que esta pasta faz
- Fornecer a lógica executável de backend, ferramentas CLI ou componentes de alta performance em Rust.

## 📂 Conteúdo e Arquivos Detalhados
- `audit_log.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `AuditLogRow`.
- `firewall_rule.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `FirewallRuleRow`.
- `image.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `ImageRow, ImageVersionRow`.
- `mod.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust com lógica de execução de sistema em Rust.
- `node.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `NodeRow`.
- `service.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `ServiceHealthStateRow, IdempotencyKeyRow`.
- `storage_snapshot.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `StorageSnapshotRow, NfsExportRow`.
- `user.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `UserRow, RefreshTokenRow, ActiveSessionRow` e funções `id, role`.

## 🔄 Histórico de Mudanças Comportamentais
- **[Inicial]**: Mapeamento e documentação detalhada da estrutura inicial do módulo.
