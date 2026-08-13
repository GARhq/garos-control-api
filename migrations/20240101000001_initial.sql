-- Initial schema for garos-backend (SQLite + Postgres compatible).

-- USERS
CREATE TABLE IF NOT EXISTS users (
    id                  TEXT PRIMARY KEY NOT NULL,
    username            TEXT NOT NULL UNIQUE,
    email               TEXT UNIQUE,
    display_name        TEXT,
    password_hash       TEXT,
    role                TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('user','operator','admin')),
    status              TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','blocked','pending','disabled')),
    quota_used_bytes    BIGINT NOT NULL DEFAULT 0,
    quota_limit_bytes   BIGINT,
    failed_login_count  INTEGER NOT NULL DEFAULT 0,
    locked_until        TIMESTAMP,
    force_password_change BOOLEAN NOT NULL DEFAULT 0,
    samba_dn            TEXT,
    created_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at          TIMESTAMP,
    last_activity_at    TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_users_email    ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_status   ON users(status);

-- REFRESH TOKENS
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id          TEXT PRIMARY KEY NOT NULL,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,
    expires_at  TIMESTAMP NOT NULL,
    revoked_at  TIMESTAMP,
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_refresh_user ON refresh_tokens(user_id);

-- NODES (diskless stations)
CREATE TABLE IF NOT EXISTS nodes (
    id                   TEXT PRIMARY KEY NOT NULL,
    mac                  TEXT NOT NULL UNIQUE,
    hostname             TEXT,
    ip                   TEXT,
    status               TEXT NOT NULL DEFAULT 'unknown' CHECK (status IN ('online','offline','booting','maintenance','reimaging','unknown')),
    image_id             TEXT,
    last_heartbeat_at    TIMESTAMP,
    last_seen_at         TIMESTAMP,
    cpu_temp_c           REAL,
    cpu_usage_pct        REAL,
    mem_usage_pct        REAL,
    fan_rpm              INTEGER,
    ping_ms              REAL,
    nfs_latency_ms       REAL,
    hardware_model       TEXT,
    serial               TEXT,
    location             TEXT,
    current_user_id      TEXT,
    current_user_role    TEXT,
    login_at             TIMESTAMP,
    created_at           TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at           TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_nodes_status  ON nodes(status);
CREATE INDEX IF NOT EXISTS idx_nodes_imageid ON nodes(image_id);
CREATE INDEX IF NOT EXISTS idx_nodes_host    ON nodes(hostname);

-- IMAGES
CREATE TABLE IF NOT EXISTS images (
    id              TEXT PRIMARY KEY NOT NULL,
    name            TEXT NOT NULL UNIQUE,
    description     TEXT,
    nixos_version   TEXT,
    kernel          TEXT,
    kernel_args     TEXT,
    size_mb         BIGINT,
    status          TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','building','ready','failed','published','deprecated')),
    packages_json   TEXT,
    custom_nix      TEXT,
    author_id       TEXT REFERENCES users(id) ON DELETE SET NULL,
    version         TEXT NOT NULL DEFAULT '1.0.0',
    parent_id       TEXT REFERENCES images(id) ON DELETE SET NULL,
    build_log       TEXT,
    published_at    TIMESTAMP,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_images_status ON images(status);

-- IMAGE VERSIONS
CREATE TABLE IF NOT EXISTS image_versions (
    id          TEXT PRIMARY KEY NOT NULL,
    image_id    TEXT NOT NULL REFERENCES images(id) ON DELETE CASCADE,
    version     TEXT NOT NULL,
    size_mb     BIGINT,
    packages_json TEXT,
    custom_nix  TEXT,
    change_summary TEXT,
    author_id   TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_image_versions_image ON image_versions(image_id);

-- FIREWALL RULES
CREATE TABLE IF NOT EXISTS firewall_rules (
    id          TEXT PRIMARY KEY NOT NULL,
    action      TEXT NOT NULL CHECK (action IN ('accept','drop','reject')),
    family      TEXT NOT NULL DEFAULT 'inet',
    table_name  TEXT NOT NULL DEFAULT 'garos',
    chain       TEXT NOT NULL DEFAULT 'input',
    protocol    TEXT,
    port        INTEGER,
    port_end    INTEGER,
    source      TEXT,
    destination TEXT,
    interface_in  TEXT,
    interface_out TEXT,
    description TEXT,
    enabled     BOOLEAN NOT NULL DEFAULT 1,
    nft_handle  TEXT,
    priority    INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by  TEXT REFERENCES users(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_fw_enabled ON firewall_rules(enabled);
CREATE INDEX IF NOT EXISTS idx_fw_priority ON firewall_rules(priority);

-- STORAGE SNAPSHOTS
CREATE TABLE IF NOT EXISTS storage_snapshots (
    id              TEXT PRIMARY KEY NOT NULL,
    pool            TEXT NOT NULL,
    subvolume       TEXT NOT NULL,
    name            TEXT NOT NULL,
    size_bytes      BIGINT NOT NULL DEFAULT 0,
    read_only       BOOLEAN NOT NULL DEFAULT 0,
    retention_until TIMESTAMP,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by      TEXT REFERENCES users(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_snap_pool ON storage_snapshots(pool);

-- NFS EXPORTS
CREATE TABLE IF NOT EXISTS nfs_exports (
    id              TEXT PRIMARY KEY NOT NULL,
    path            TEXT NOT NULL UNIQUE,
    allowed_clients TEXT NOT NULL,
    options         TEXT NOT NULL DEFAULT 'ro,sync,no_subtree_check',
    writable        BOOLEAN NOT NULL DEFAULT 0,
    sync            BOOLEAN NOT NULL DEFAULT 1,
    enabled         BOOLEAN NOT NULL DEFAULT 1,
    description     TEXT,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- AUDIT LOG
CREATE TABLE IF NOT EXISTS audit_log (
    id            TEXT PRIMARY KEY NOT NULL,
    actor_id      TEXT,
    actor_username TEXT,
    action        TEXT NOT NULL,
    target_type   TEXT,
    target_id     TEXT,
    before_json   TEXT,
    after_json    TEXT,
    ip            TEXT,
    user_agent    TEXT,
    trace_id      TEXT,
    result        TEXT NOT NULL DEFAULT 'success' CHECK (result IN ('success','failure','denied')),
    error_message TEXT,
    created_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_log(actor_id, created_at);
CREATE INDEX IF NOT EXISTS idx_audit_target ON audit_log(target_type, target_id, created_at);
CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_log(action, created_at);

-- IDEMPOTENCY KEYS
CREATE TABLE IF NOT EXISTS idempotency_keys (
    key           TEXT NOT NULL,
    user_id       TEXT NOT NULL,
    method        TEXT NOT NULL,
    path          TEXT NOT NULL,
    request_hash  TEXT NOT NULL,
    status        INTEGER NOT NULL,
    response_json TEXT NOT NULL,
    created_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at    TIMESTAMP NOT NULL,
    PRIMARY KEY (key, user_id)
);
CREATE INDEX IF NOT EXISTS idx_idem_expires ON idempotency_keys(expires_at);

-- SERVICE HEALTH STATE
CREATE TABLE IF NOT EXISTS service_health_state (
    service_name         TEXT PRIMARY KEY,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    last_failure_at      TIMESTAMP,
    last_success_at      TIMESTAMP,
    needs_attention      BOOLEAN NOT NULL DEFAULT 0,
    last_status_json     TEXT
);

-- ACTIVE SESSIONS
CREATE TABLE IF NOT EXISTS active_sessions (
    id           TEXT PRIMARY KEY NOT NULL,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ip           TEXT,
    user_agent   TEXT,
    login_at     TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at   TIMESTAMP NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON active_sessions(user_id);
