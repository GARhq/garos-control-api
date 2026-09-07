# domain

## 📌 Visão Geral
Módulo integrante da arquitetura do repositório GAROS no caminho `garos-control-api/src/domain`.

## ⚙️ O que esta pasta faz
- Fornecer a lógica executável de backend, ferramentas CLI ou componentes de alta performance em Rust.

## 📂 Conteúdo e Arquivos Detalhados
- `audit.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `ActivityEvent, AuditEntry, AuditQuery`.
- `firewall.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `FirewallRuleView, FirewallRuleCreate, FirewallRuleUpdate`.
- `image.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `ImageView, ImageCreate, ImageUpdate`.
- `mod.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust com lógica de execução de sistema em Rust.
- `node.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `NetbootDevice, MacAddress, HeartbeatRequest` e funções `fmt, is_valid`.
- `service.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `ServiceView, ServiceHealth, LogLine`.
- `storage.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `StoragePool, ScrubStatus, Snapshot`.
- `user.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `UserCreate, UserUpdate, UserView`.

## 🔄 Histórico de Mudanças Comportamentais
- **[Inicial]**: Mapeamento e documentação detalhada da estrutura inicial do módulo.
