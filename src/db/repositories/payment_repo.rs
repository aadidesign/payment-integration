use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{
    CreatePaymentRequest, CurrencyType, Payment, PaymentMethod, PaymentStatus,
};

pub struct PaymentRepository;

impl PaymentRepository {
    pub async fn create(
        pool: &PgPool,
        request: &CreatePaymentRequest,
    ) -> AppResult<Payment> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let payment = sqlx::query_as!(
            Payment,
            r#"
            INSERT INTO payments (
                id, amount, currency, status, method, description,
                customer_email, customer_phone, metadata, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING
                id, external_id, order_id, amount,
                currency as "currency: CurrencyType",
                status as "status: PaymentStatus",
                method as "method: PaymentMethod",
                description, customer_email, customer_phone, metadata,
                razorpay_payment_id, razorpay_order_id, razorpay_signature,
                crypto_tx_hash, crypto_from_address, crypto_to_address, crypto_chain,
                lightning_invoice, lightning_payment_hash,
                expires_at, completed_at, created_at, updated_at
            "#,
            id,
            request.amount,
            request.currency.clone() as CurrencyType,
            PaymentStatus::Pending as PaymentStatus,
            request.method.clone() as PaymentMethod,
            request.description,
            request.customer_email,
            request.customer_phone,
            request.metadata,
            now,
            now
        )
        .fetch_one(pool)
        .await?;

        Ok(payment)
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> AppResult<Payment> {
        let payment = sqlx::query_as!(
            Payment,
            r#"
            SELECT
                id, external_id, order_id, amount,
                currency as "currency: CurrencyType",
                status as "status: PaymentStatus",
                method as "method: PaymentMethod",
                description, customer_email, customer_phone, metadata,
                razorpay_payment_id, razorpay_order_id, razorpay_signature,
                crypto_tx_hash, crypto_from_address, crypto_to_address, crypto_chain,
                lightning_invoice, lightning_payment_hash,
                expires_at, completed_at, created_at, updated_at
            FROM payments
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Payment {} not found", id)))?;

        Ok(payment)
    }

    pub async fn find_by_razorpay_order_id(
        pool: &PgPool,
        order_id: &str,
    ) -> AppResult<Option<Payment>> {
        let payment = sqlx::query_as!(
            Payment,
            r#"
            SELECT
                id, external_id, order_id, amount,
                currency as "currency: CurrencyType",
                status as "status: PaymentStatus",
                method as "method: PaymentMethod",
                description, customer_email, customer_phone, metadata,
                razorpay_payment_id, razorpay_order_id, razorpay_signature,
                crypto_tx_hash, crypto_from_address, crypto_to_address, crypto_chain,
                lightning_invoice, lightning_payment_hash,
                expires_at, completed_at, created_at, updated_at
            FROM payments
            WHERE razorpay_order_id = $1
            "#,
            order_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(payment)
    }

    pub async fn update_status(
        pool: &PgPool,
        id: Uuid,
        status: PaymentStatus,
    ) -> AppResult<Payment> {
        let completed_at = if status == PaymentStatus::Completed {
            Some(Utc::now())
        } else {
            None
        };

        let payment = sqlx::query_as!(
            Payment,
            r#"
            UPDATE payments
            SET status = $2, completed_at = COALESCE($3, completed_at), updated_at = $4
            WHERE id = $1
            RETURNING
                id, external_id, order_id, amount,
                currency as "currency: CurrencyType",
                status as "status: PaymentStatus",
                method as "method: PaymentMethod",
                description, customer_email, customer_phone, metadata,
                razorpay_payment_id, razorpay_order_id, razorpay_signature,
                crypto_tx_hash, crypto_from_address, crypto_to_address, crypto_chain,
                lightning_invoice, lightning_payment_hash,
                expires_at, completed_at, created_at, updated_at
            "#,
            id,
            status as PaymentStatus,
            completed_at,
            Utc::now()
        )
        .fetch_one(pool)
        .await?;

        Ok(payment)
    }

    pub async fn update_razorpay_details(
        pool: &PgPool,
        id: Uuid,
        razorpay_order_id: &str,
        razorpay_payment_id: Option<&str>,
        razorpay_signature: Option<&str>,
    ) -> AppResult<Payment> {
        let payment = sqlx::query_as!(
            Payment,
            r#"
            UPDATE payments
            SET razorpay_order_id = $2, razorpay_payment_id = $3,
                razorpay_signature = $4, updated_at = $5
            WHERE id = $1
            RETURNING
                id, external_id, order_id, amount,
                currency as "currency: CurrencyType",
                status as "status: PaymentStatus",
                method as "method: PaymentMethod",
                description, customer_email, customer_phone, metadata,
                razorpay_payment_id, razorpay_order_id, razorpay_signature,
                crypto_tx_hash, crypto_from_address, crypto_to_address, crypto_chain,
                lightning_invoice, lightning_payment_hash,
                expires_at, completed_at, created_at, updated_at
            "#,
            id,
            razorpay_order_id,
            razorpay_payment_id,
            razorpay_signature,
            Utc::now()
        )
        .fetch_one(pool)
        .await?;

        Ok(payment)
    }

    pub async fn update_crypto_details(
        pool: &PgPool,
        id: Uuid,
        tx_hash: Option<&str>,
        from_address: Option<&str>,
        to_address: Option<&str>,
        chain: Option<&str>,
    ) -> AppResult<Payment> {
        let payment = sqlx::query_as!(
            Payment,
            r#"
            UPDATE payments
            SET crypto_tx_hash = COALESCE($2, crypto_tx_hash),
                crypto_from_address = COALESCE($3, crypto_from_address),
                crypto_to_address = COALESCE($4, crypto_to_address),
                crypto_chain = COALESCE($5, crypto_chain),
                updated_at = $6
            WHERE id = $1
            RETURNING
                id, external_id, order_id, amount,
                currency as "currency: CurrencyType",
                status as "status: PaymentStatus",
                method as "method: PaymentMethod",
                description, customer_email, customer_phone, metadata,
                razorpay_payment_id, razorpay_order_id, razorpay_signature,
                crypto_tx_hash, crypto_from_address, crypto_to_address, crypto_chain,
                lightning_invoice, lightning_payment_hash,
                expires_at, completed_at, created_at, updated_at
            "#,
            id,
            tx_hash,
            from_address,
            to_address,
            chain,
            Utc::now()
        )
        .fetch_one(pool)
        .await?;

        Ok(payment)
    }

    pub async fn update_lightning_details(
        pool: &PgPool,
        id: Uuid,
        invoice: &str,
        payment_hash: &str,
    ) -> AppResult<Payment> {
        let payment = sqlx::query_as!(
            Payment,
            r#"
            UPDATE payments
            SET lightning_invoice = $2, lightning_payment_hash = $3, updated_at = $4
            WHERE id = $1
            RETURNING
                id, external_id, order_id, amount,
                currency as "currency: CurrencyType",
                status as "status: PaymentStatus",
                method as "method: PaymentMethod",
                description, customer_email, customer_phone, metadata,
                razorpay_payment_id, razorpay_order_id, razorpay_signature,
                crypto_tx_hash, crypto_from_address, crypto_to_address, crypto_chain,
                lightning_invoice, lightning_payment_hash,
                expires_at, completed_at, created_at, updated_at
            "#,
            id,
            invoice,
            payment_hash,
            Utc::now()
        )
        .fetch_one(pool)
        .await?;

        Ok(payment)
    }

    pub async fn find_pending_by_crypto_address(
        pool: &PgPool,
        address: &str,
        chain: &str,
    ) -> AppResult<Option<Payment>> {
        let payment = sqlx::query_as!(
            Payment,
            r#"
            SELECT
                id, external_id, order_id, amount,
                currency as "currency: CurrencyType",
                status as "status: PaymentStatus",
                method as "method: PaymentMethod",
                description, customer_email, customer_phone, metadata,
                razorpay_payment_id, razorpay_order_id, razorpay_signature,
                crypto_tx_hash, crypto_from_address, crypto_to_address, crypto_chain,
                lightning_invoice, lightning_payment_hash,
                expires_at, completed_at, created_at, updated_at
            FROM payments
            WHERE crypto_to_address = $1
                AND crypto_chain = $2
                AND status = 'pending'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            address,
            chain
        )
        .fetch_optional(pool)
        .await?;

        Ok(payment)
    }
}
