use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::PgPool;
use std::sync::Arc;

use crate::error::AppResult;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub database: String,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub version: String,
    pub services: ServiceStatus,
    pub uptime_seconds: u64,
}

#[derive(Serialize)]
pub struct ServiceStatus {
    pub database: bool,
    pub razorpay: bool,
    pub ethereum: bool,
    pub solana: bool,
    pub lightning: bool,
}

pub async fn health_check(State(pool): State<Arc<PgPool>>) -> AppResult<Json<HealthResponse>> {
    // Check database connection
    let db_status = sqlx::query("SELECT 1")
        .execute(pool.as_ref())
        .await
        .map(|_| "connected")
        .unwrap_or("disconnected");

    Ok(Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_status.to_string(),
    }))
}

pub async fn service_status(
    State(pool): State<Arc<PgPool>>,
) -> AppResult<Json<StatusResponse>> {
    // Check database
    let db_ok = sqlx::query("SELECT 1")
        .execute(pool.as_ref())
        .await
        .is_ok();

    // In production, you would check each service's health
    let services = ServiceStatus {
        database: db_ok,
        razorpay: true, // Would ping Razorpay API
        ethereum: true, // Would check RPC connection
        solana: true,   // Would check RPC connection
        lightning: true, // Would check node connection
    };

    Ok(Json(StatusResponse {
        status: if db_ok { "healthy" } else { "degraded" }.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        services,
        uptime_seconds: 0, // Would track actual uptime
    }))
}
