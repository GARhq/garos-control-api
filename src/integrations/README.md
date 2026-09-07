# integrations

## 📌 Visão Geral
Módulo integrante da arquitetura do repositório GAROS no caminho `garos-control-api/src/integrations`.

## ⚙️ O que esta pasta faz
- Fornecer a lógica executável de backend, ferramentas CLI ou componentes de alta performance em Rust.

## 📂 Conteúdo e Arquivos Detalhados
- `btrfs.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `BtrfsIntegration` e funções `pools, usage, start_scrub`.
- `journald.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `JournaldIntegration` e funções `query, stream, new`.
- `local_sso.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `LocalSsoUser, LocalSsoGroup, LocalSsoOu` e funções `list_users, get_user, create_user`.
- `mod.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust com lógica de execução de sistema em Rust.
- `nftables.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `NftablesIntegration` e funções `list_ruleset, add_rule, delete_rule`.
- `nix.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `NixBuildResult, NixIntegration` e funções `validate_arg, run_nix, nix_build`.
- `pxe.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `PxeIntegration` e funções `render_menu, render_per_host, write_menu`.
- `samba.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `SambaUser, SambaGroup, SambaOu` e funções `list_users, get_user, create_user`.
- `systemd.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `SystemdIntegration` e funções `list_units, unit, start`.
- `wol.rs`: **[Código-Fonte Rust]** — Módulo de alta performance em Rust, contendo structs `WolReceipt, WolIntegration` e funções `send, send_many, new`.

## 🔄 Histórico de Mudanças Comportamentais
- **[Inicial]**: Mapeamento e documentação detalhada da estrutura inicial do módulo.
