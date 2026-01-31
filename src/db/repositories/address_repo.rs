use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{ChainType, CryptoAddress};

pub struct AddressRepository;

impl AddressRepository {
    pub async fn create(
        pool: &PgPool,
        address: &str,
        chain: ChainType,
        payment_id: Option<Uuid>,
        expected_amount: Option<i64>,
        label: Option<&str>,
        token_address: Option<&str>,
    ) -> AppResult<CryptoAddress> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let addr = sqlx::query_as!(
            CryptoAddress,
            r#"
            INSERT INTO crypto_addresses (
                id, address, chain, payment_id, expected_amount,
                label, token_address, is_active, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING
                id, payment_id, address,
                chain as "chain: ChainType",
                is_active, label, expected_amount, received_amount,
                token_address, last_checked_at, created_at, updated_at
            "#,
            id,
            address,
            chain as ChainType,
            payment_id,
            expected_amount,
            label,
            token_address,
            true,
            now,
            now
        )
        .fetch_one(pool)
        .await?;

        Ok(addr)
    }

    pub async fn find_by_address(
        pool: &PgPool,
        address: &str,
        chain: ChainType,
    ) -> AppResult<Option<CryptoAddress>> {
        let addr = sqlx::query_as!(
            CryptoAddress,
            r#"
            SELECT
                id, payment_id, address,
                chain as "chain: ChainType",
                is_active, label, expected_amount, received_amount,
                token_address, last_checked_at, created_at, updated_at
            FROM crypto_addresses
            WHERE address = $1 AND chain = $2
            "#,
            address,
            chain as ChainType
        )
        .fetch_optional(pool)
        .await?;

        Ok(addr)
    }

    pub async fn find_by_payment_id(
        pool: &PgPool,
        payment_id: Uuid,
    ) -> AppResult<Option<CryptoAddress>> {
        let addr = sqlx::query_as!(
            CryptoAddress,
            r#"
            SELECT
                id, payment_id, address,
                chain as "chain: ChainType",
                is_active, label, expected_amount, received_amount,
                token_address, last_checked_at, created_at, updated_at
            FROM crypto_addresses
            WHERE payment_id = $1
            "#,
            payment_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(addr)
    }

    pub async fn find_active_for_monitoring(
        pool: &PgPool,
        chain: ChainType,
    ) -> AppResult<Vec<CryptoAddress>> {
        let addrs = sqlx::query_as!(
            CryptoAddress,
            r#"
            SELECT
                id, payment_id, address,
                chain as "chain: ChainType",
                is_active, label, expected_amount, received_amount,
                token_address, last_checked_at, created_at, updated_at
            FROM crypto_addresses
            WHERE chain = $1 AND is_active = true
            ORDER BY created_at DESC
            "#,
            chain as ChainType
        )
        .fetch_all(pool)
        .await?;

        Ok(addrs)
    }

    pub async fn update_received_amount(
        pool: &PgPool,
        id: Uuid,
        received_amount: i64,
    ) -> AppResult<CryptoAddress> {
        let addr = sqlx::query_as!(
            CryptoAddress,
            r#"
            UPDATE crypto_addresses
            SET received_amount = $2, last_checked_at = $3, updated_at = $4
            WHERE id = $1
            RETURNING
                id, payment_id, address,
                chain as "chain: ChainType",
                is_active, label, expected_amount, received_amount,
                token_address, last_checked_at, created_at, updated_at
            "#,
            id,
            received_amount,
            Utc::now(),
            Utc::now()
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Address {} not found", id)))?;

        Ok(addr)
    }

    pub async fn deactivate(pool: &PgPool, id: Uuid) -> AppResult<()> {
        sqlx::query!(
            r#"
            UPDATE crypto_addresses
            SET is_active = false, updated_at = $2
            WHERE id = $1
            "#,
            id,
            Utc::now()
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
