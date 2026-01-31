use std::net::SocketAddr;

use payment_gateway::{
    api::create_router,
    api::middleware::logging::init_tracing,
    config::Config,
    db::{create_pool, run_migrations},
    services::PaymentProcessor,
    websocket::PaymentBroadcaster,
    AppState,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize logging
    init_tracing();

    tracing::info!("Starting Payment Gateway API v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = Config::from_env()
        .map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;

    tracing::info!("Configuration loaded successfully");

    // Create database connection pool
    let db_pool = create_pool(&config.database).await?;

    tracing::info!("Database connection pool created");

    // Run migrations
    run_migrations(&db_pool).await?;

    tracing::info!("Database migrations completed");

    // Initialize payment processor
    let payment_processor = PaymentProcessor::new(&config).await?;

    tracing::info!("Payment processor initialized");

    // Initialize WebSocket broadcaster
    let ws_broadcaster = PaymentBroadcaster::new();

    tracing::info!("WebSocket broadcaster initialized");

    // Create application state
    let state = AppState::new(
        config.clone(),
        (*db_pool).clone(),
        payment_processor,
        Some(ws_broadcaster),
    );

    // Create router
    let app = create_router(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));

    tracing::info!("Server starting on http://{}", addr);
    tracing::info!("Health check: http://{}/health", addr);
    tracing::info!("API documentation: http://{}/api/v1/status", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("Payment Gateway API is ready to accept connections");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shutdown complete");

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, starting graceful shutdown...");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, starting graceful shutdown...");
        },
    }
}
