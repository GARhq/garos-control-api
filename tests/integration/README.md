# integration

## 📌 Visão Geral
Módulo integrante da arquitetura do repositório GAROS no caminho `garos-control-api/tests/integration`.

## ⚙️ O que esta pasta faz
- Fornecer a lógica executável de backend, ferramentas CLI ou componentes de alta performance em Rust.

## 📂 Conteúdo e Arquivos Detalhados
- `auth_test.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo funções `admin_can_log_in, wrong_password_is_unauthorized`.
- `firewall_test.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo funções `create_and_validate`.
- `health_test.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo funções `health_db_works`.
- `images_test.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo funções `create_and_publish`.
- `mod.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust com lógica de execução de sistema em Rust.
- `nodes_test.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo funções `heartbeat_creates_node, bulk_wol_reports_each`.
- `users_test.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo funções `create_user_and_find`.

## 🔄 Histórico de Mudanças Comportamentais
- **[Inicial]**: Mapeamento e documentação detalhada da estrutura inicial do módulo.
