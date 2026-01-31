use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{Transaction, TransactionStatus, TransactionType};

pub struct TransactionRepository;

impl TransactionRepository {
    pub async fn create(
        pool: &PgPool,
        payment_id: Uuid,
        tx_type: TransactionType,
        amount: i64,
        currency: &str,
        chain: Option<&str>,
        required_confirmations: i32,
    ) -> AppResult<Transaction> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let tx = sqlx::query_as!(
            Transaction,
            r#"
            INSERT INTO transactions (
                id, payment_id, tx_type, status, amount, currency, chain,
                confirmations, required_confirmations, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING
                id, payment_id,
                tx_type as "tx_type: TransactionType",
                status as "status: TransactionStatus",
                amount, fee, currency, tx_hash, block_number,
                confirmations, required_confirmations,
                from_address, to_address, chain, raw_data,
                error_message, created_at, updated_at
            "#,
            id,
            payment_id,
            tx_type as TransactionType,
            TransactionStatus::Pending as TransactionStatus,
            amount,
            currency,
            chain,
            0_i32,
            required_confirmations,
            now,
            now
        )
        .fetch_one(pool)
        .await?;

        Ok(tx)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> AppResult<Transaction> {
        let tx = sqlx::query_as!(
            Transaction,
            r#"
            SELECT
                id, payment_id,
                tx_type as "tx_type: TransactionType",
                status as "status: TransactionStatus",
                amount, fee, currency, tx_hash, block_number,
                confirmations, required_confirmations,
                from_address, to_address, chain, raw_data,
                error_message, created_at, updated_at
            FROM transactions
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Transaction {} not found", id)))?;

        Ok(tx)
    }

    pub async fn find_by_payment_id(pool: &PgPool, payment_id: Uuid) -> AppResult<Vec<Transaction>> {
        let txs = sqlx::query_as!(
            Transaction,
            r#"
            SELECT
                id, payment_id,
                tx_type as "tx_type: TransactionType",
                status as "status: TransactionStatus",
                amount, fee, currency, tx_hash, block_number,
                confirmations, required_confirmations,
                from_address, to_address, chain, raw_data,
                error_message, created_at, updated_at
            FROM transactions
            WHERE payment_id = $1
            ORDER BY created_at DESC
            "#,
            payment_id
        )
        .fetch_all(pool)
        .await?;

        Ok(txs)
    }

    pub async fn find_by_tx_hash(pool: &PgPool, tx_hash: &str) -> AppResult<Option<Transaction>> {
        let tx = sqlx::query_as!(
            Transaction,
            r#"
            SELECT
                id, payment_id,
                tx_type as "tx_type: TransactionType",
                status as "status: TransactionStatus",
                amount, fee, currency, tx_hash, block_number,
                confirmations, required_confirmations,
                from_address, to_address, chain, raw_data,
                error_message, created_at, updated_at
            FROM transactions
            WHERE tx_hash = $1
            "#,
            tx_hash
        )
        .fetch_optional(pool)
        .await?;

        Ok(tx)
    }

    pub async fn update_status(
        pool: &PgPool,
        id: Uuid,
        status: TransactionStatus,
        error_message: Option<&str>,
    ) -> AppResult<Transaction> {
        let tx = sqlx::query_as!(
            Transaction,
            r#"
            UPDATE transactions
            SET status = $2, error_message = $3, updated_at = $4
            WHERE id = $1
            RETURNING
                id, payment_id,
                tx_type as "tx_type: TransactionType",
                status as "status: TransactionStatus",
                amount, fee, currency, tx_hash, block_number,
                confirmations, required_confirmations,
                from_address, to_address, chain, raw_data,
                error_message, created_at, updated_at
            "#,
            id,
            status as TransactionStatus,
            error_message,
            Utc::now()
        )
        .fetch_one(pool)
        .await?;

        Ok(tx)
    }

    pub async fn update_blockchain_details(
        pool: &PgPool,
        id: Uuid,
        tx_hash: &str,
        block_number: Option<i64>,
        from_address: Option<&str>,
        to_address: Option<&str>,
        fee: Option<i64>,
    ) -> AppResult<Transaction> {
        let tx = sqlx::query_as!(
            Transaction,
            r#"
            UPDATE transactions
            SET tx_hash = $2, block_number = $3, from_address = $4,
                to_address = $5, fee = $6, updated_at = $7
            WHERE id = $1
            RETURNING
                id, payment_id,
                tx_type as "tx_type: TransactionType",
                status as "status: TransactionStatus",
                amount, fee, currency, tx_hash, block_number,
                confirmations, required_confirmations,
                from_address, to_address, chain, raw_data,
                error_message, created_at, updated_at
            "#,
            id,
            tx_hash,
            block_number,
            from_address,
            to_address,
            fee,
            Utc::now()
        )
        .fetch_one(pool)
        .await?;

        Ok(tx)
    }

    pub async fn update_confirmations(
        pool: &PgPool,
        id: Uuid,
        confirmations: i32,
        status: TransactionStatus,
    ) -> AppResult<Transaction> {
        let tx = sqlx::query_as!(
            Transaction,
            r#"
            UPDATE transactions
            SET confirmations = $2, status = $3, updated_at = $4
            WHERE id = $1
            RETURNING
                id, payment_id,
                tx_type as "tx_type: TransactionType",
                status as "status: TransactionStatus",
                amount, fee, currency, tx_hash, block_number,
                confirmations, required_confirmations,
                from_address, to_address, chain, raw_data,
                error_message, created_at, updated_at
            "#,
            id,
            confirmations,
            status as TransactionStatus,
            Utc::now()
        )
        .fetch_one(pool)
        .await?;

        Ok(tx)
    }

    pub async fn find_pending_confirmations(pool: &PgPool) -> AppResult<Vec<Transaction>> {
        let txs = sqlx::query_as!(
            Transaction,
            r#"
            SELECT
                id, payment_id,
                tx_type as "tx_type: TransactionType",
                status as "status: TransactionStatus",
                amount, fee, currency, tx_hash, block_number,
                confirmations, required_confirmations,
                from_address, to_address, chain, raw_data,
                error_message, created_at, updated_at
            FROM transactions
            WHERE status = 'confirming'
                AND tx_hash IS NOT NULL
            ORDER BY created_at ASC
            "#
        )
        .fetch_all(pool)
        .await?;

        Ok(txs)
    }
}
