use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppResult;
use crate::models::{WebhookEvent, WebhookSource, WebhookStatus};

pub struct WebhookRepository;

impl WebhookRepository {
    pub async fn create(
        pool: &PgPool,
        source: WebhookSource,
        event_type: &str,
        payload: serde_json::Value,
        headers: Option<serde_json::Value>,
        signature: Option<&str>,
    ) -> AppResult<WebhookEvent> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let event = sqlx::query_as!(
            WebhookEvent,
            r#"
            INSERT INTO webhook_events (
                id, source, event_type, payload, headers, signature,
                status, signature_verified, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id,
                source as "source: WebhookSource",
                event_type, event_id, payment_id,
                status as "status: WebhookStatus",
                payload, headers, signature, signature_verified,
                error_message, processed_at, created_at
            "#,
            id,
            source as WebhookSource,
            event_type,
            payload,
            headers,
            signature,
            WebhookStatus::Received as WebhookStatus,
            false,
            now
        )
        .fetch_one(pool)
        .await?;

        Ok(event)
    }

    pub async fn update_status(
        pool: &PgPool,
        id: Uuid,
        status: WebhookStatus,
        signature_verified: bool,
        payment_id: Option<Uuid>,
        error_message: Option<&str>,
    ) -> AppResult<WebhookEvent> {
        let processed_at = if status == WebhookStatus::Processed || status == WebhookStatus::Failed
        {
            Some(Utc::now())
        } else {
            None
        };

        let event = sqlx::query_as!(
            WebhookEvent,
            r#"
            UPDATE webhook_events
            SET status = $2, signature_verified = $3, payment_id = $4,
                error_message = $5, processed_at = $6
            WHERE id = $1
            RETURNING
                id,
                source as "source: WebhookSource",
                event_type, event_id, payment_id,
                status as "status: WebhookStatus",
                payload, headers, signature, signature_verified,
                error_message, processed_at, created_at
            "#,
            id,
            status as WebhookStatus,
            signature_verified,
            payment_id,
            error_message,
            processed_at
        )
        .fetch_one(pool)
        .await?;

        Ok(event)
    }

    pub async fn find_by_event_id(
        pool: &PgPool,
        source: WebhookSource,
        event_id: &str,
    ) -> AppResult<Option<WebhookEvent>> {
        let event = sqlx::query_as!(
            WebhookEvent,
            r#"
            SELECT
                id,
                source as "source: WebhookSource",
                event_type, event_id, payment_id,
                status as "status: WebhookStatus",
                payload, headers, signature, signature_verified,
                error_message, processed_at, created_at
            FROM webhook_events
            WHERE source = $1 AND event_id = $2
            "#,
            source as WebhookSource,
            event_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(event)
    }

    pub async fn find_recent_by_payment_id(
        pool: &PgPool,
        payment_id: Uuid,
        limit: i64,
    ) -> AppResult<Vec<WebhookEvent>> {
        let events = sqlx::query_as!(
            WebhookEvent,
            r#"
            SELECT
                id,
                source as "source: WebhookSource",
                event_type, event_id, payment_id,
                status as "status: WebhookStatus",
                payload, headers, signature, signature_verified,
                error_message, processed_at, created_at
            FROM webhook_events
            WHERE payment_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
            payment_id,
            limit
        )
        .fetch_all(pool)
        .await?;

        Ok(events)
    }
}
