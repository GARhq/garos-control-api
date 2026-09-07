# services

## 📌 Visão Geral
Módulo integrante da arquitetura do repositório GAROS no caminho `garos-control-api/src/services`.

## ⚙️ O que esta pasta faz
- Fornecer a lógica executável de backend, ferramentas CLI ou componentes de alta performance em Rust.

## 📂 Conteúdo e Arquivos Detalhados
- `audit_service.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `AuditService` e funções `new, repo, list`.
- `firewall_service.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `FirewallService` e funções `new, repo, list`.
- `image_service.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `ImageService` e funções `new, repo, list`.
- `mod.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust com lógica de execução de sistema em Rust.
- `node_service.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `NodeService` e funções `new, repo, audit`.
- `service_manager.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `ServiceManager` e funções `new, list, by_name`.
- `storage_service.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `StorageService` e funções `new, repo, pools`.
- `user_service.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `UserService` e funções `new, jwt, repo`.

## 🔄 Histórico de Mudanças Comportamentais
- **[Inicial]**: Mapeamento e documentação detalhada da estrutura inicial do módulo.
