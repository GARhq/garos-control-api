# repositories

## 📌 Visão Geral
Módulo integrante da arquitetura do repositório GAROS no caminho `garos-control-api/src/db/repositories`.

## ⚙️ O que esta pasta faz
- Fornecer a lógica executável de backend, ferramentas CLI ou componentes de alta performance em Rust.

## 📂 Conteúdo e Arquivos Detalhados
- `audit.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `AuditRepo` e funções `new, record, list`.
- `firewall.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `FirewallRepo` e funções `new, list, by_id`.
- `images.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `ImageRepo` e funções `new, by_id, by_name`.
- `mod.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust com lógica de execução de sistema em Rust.
- `nodes.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `NodeRepo` e funções `new, by_id, by_mac`.
- `services.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `ServiceHealthRepo` e funções `new, list, by_name`.
- `storage.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `StorageRepo` e funções `new, list_snapshots, snapshot_by_id`.
- `users.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `UserRepo` e funções `new, pool, count`.

## 🔄 Histórico de Mudanças Comportamentais
- **[Inicial]**: Mapeamento e documentação detalhada da estrutura inicial do módulo.
