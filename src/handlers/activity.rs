//! Activity feed.

use crate::auth::extractor::CurrentUser;
use crate::domain::audit::ActivityEvent;
use crate::error::AppError;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ActivityQuery {
    pub since: Option<String>,
    pub limit: Option<i64>,
}

/// `GET /api/garos/activity`
pub async fn feed(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(q): Query<ActivityQuery>,
) -> Result<Json<Vec<ActivityEvent>>, AppError> {
    let limit = q.limit.unwrap_or(20).clamp(1, 200);
    let rows = state
        .audit
        .list(crate::domain::audit::AuditQuery {
            actor: None,
            action: None,
            target: None,
            from: None,
            to: None,
            limit: Some(limit),
            offset: Some(0),
        })
        .await?;
    let feed: Vec<ActivityEvent> = rows
        .into_iter()
        .map(|e| ActivityEvent {
            id: Uuid::parse_str(&e.id.to_string()).unwrap_or_else(|_| Uuid::nil()),
            kind: e.action.clone(),
            title: e.action.clone(),
            description: e.error_message,
            actor: e.actor_username,
            target: e.target_id,
            severity: if e.result == "success" {
                "info".into()
            } else {
                "warn".into()
            },
            at: e.created_at,
        })
        .collect();
    Ok(Json(feed))
}
