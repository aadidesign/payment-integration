# Payment Gateway API

A production-ready, high-performance payment gateway backend built in Rust. Supports traditional payments via Razorpay and cryptocurrency payments via direct blockchain integration.

## Features

### Payment Methods

**Traditional Payments (via Razorpay)**
- Credit/Debit Cards
- UPI (Unified Payments Interface)
- Net Banking
- Wallets (PayTM, PhonePe, etc.)
- EMI

**Cryptocurrency Payments (Direct Blockchain Integration)**
- **Ethereum & EVM Chains**: Ethereum, Polygon, BSC, Arbitrum
- **Solana**: Native SOL and SPL tokens
- **Bitcoin**: Lightning Network

**Wallet Support**
- MetaMask
- Phantom
- WalletConnect
- Trust Wallet

### Technical Features

- **High Performance**: Built with Rust and Axum for maximum throughput
- **Real-time Updates**: WebSocket support for payment status notifications
- **Multi-chain**: Support for multiple blockchain networks
- **Secure**: HMAC authentication, signature verification, rate limiting
- **Production Ready**: Docker support, health checks, graceful shutdown

## Prerequisites

- Rust 1.75+ (for building)
- PostgreSQL 14+
- Docker & Docker Compose (for containerized deployment)

### For Blockchain Integration
- Ethereum RPC endpoint (Infura, Alchemy, or self-hosted)
- Solana RPC endpoint
- Lightning Network node (optional, for BTC payments)

### Windows Development (Optional)
For local Windows development, you'll need OpenSSL installed:
```bash
# Using vcpkg (recommended)
vcpkg install openssl:x64-windows
set OPENSSL_DIR=C:\vcpkg\installed\x64-windows

# Or use pre-built binaries from:
# https://slproweb.com/products/Win32OpenSSL.html
```

**Recommended**: Use Docker for building and testing instead of local Windows development.

## Quick Start

### 1. Clone and Configure

```bash
# Copy environment template
cp .env.example .env

# Edit .env with your configuration
# Required: DATABASE_URL, RAZORPAY_KEY_ID, RAZORPAY_KEY_SECRET
```

### 2. Run with Docker (Recommended)

```bash
cd docker
docker-compose up -d
```

### 3. Run Locally (Development)

```bash
# Install sqlx-cli for migrations
cargo install sqlx-cli

# Create database
createdb payment_gateway

# Run migrations
sqlx migrate run

# Start the server
cargo run
```

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `DATABASE_URL` | PostgreSQL connection string | Yes |
| `RAZORPAY_KEY_ID` | Razorpay API Key ID | Yes |
| `RAZORPAY_KEY_SECRET` | Razorpay API Key Secret | Yes |
| `RAZORPAY_WEBHOOK_SECRET` | Razorpay Webhook Secret | Yes |
| `ETH_RPC_URL` | Ethereum RPC endpoint | Yes |
| `SOLANA_RPC_URL` | Solana RPC endpoint | Yes |
| `API_KEY_HASH_SECRET` | Secret for API key hashing | Yes |
| `JWT_SECRET` | JWT signing secret | Yes |
| `ENCRYPTION_KEY` | 32-byte encryption key | Yes |

See `.env.example` for complete list.

## API Reference

### Health & Status

```
GET /health              - Health check
GET /api/v1/status       - Service status
```

### Razorpay Payments

```
POST /api/v1/razorpay/orders       - Create order
POST /api/v1/razorpay/verify       - Verify payment
GET  /api/v1/razorpay/payments/:id - Get payment
POST /api/v1/razorpay/refund       - Process refund
```

### Crypto Payments

```
POST /api/v1/crypto/payment        - Create crypto payment
GET  /api/v1/crypto/payment/:id    - Get payment status
POST /api/v1/crypto/verify         - Verify transaction
GET  /api/v1/crypto/balance        - Get wallet balance
POST /api/v1/crypto/verify-signature - Verify wallet signature
```

### Webhooks

```
POST /webhooks/razorpay            - Razorpay webhook
POST /webhooks/blockchain          - Blockchain event webhook
```

### WebSocket

```
WS /ws/payments                    - Real-time payment updates
```

## Usage Examples

### Create Razorpay Order

```bash
curl -X POST http://localhost:8080/api/v1/razorpay/orders \
  -H "Content-Type: application/json" \
  -H "X-API-Key: pk_test_your_key" \
  -d '{
    "amount": 50000,
    "currency": "INR",
    "description": "Test payment",
    "customer_email": "customer@example.com"
  }'
```

### Create Crypto Payment

```bash
curl -X POST http://localhost:8080/api/v1/crypto/payment \
  -H "Content-Type: application/json" \
  -H "X-API-Key: pk_test_your_key" \
  -d '{
    "amount": 1000000000000000000,
    "currency": "ETH",
    "chain": "ethereum",
    "description": "1 ETH payment"
  }'
```

### Verify Wallet Signature

```bash
curl -X POST http://localhost:8080/api/v1/crypto/verify-signature \
  -H "Content-Type: application/json" \
  -H "X-API-Key: pk_test_your_key" \
  -d '{
    "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f1E8e4",
    "message": "Sign this message",
    "signature": "0x...",
    "chain": "ethereum"
  }'
```

### WebSocket Connection

```javascript
const ws = new WebSocket('ws://localhost:8080/ws/payments');

ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'subscribe',
    data: { payment_ids: ['uuid-here'] }
  }));
};

ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  console.log('Payment update:', update);
};
```

## Security Considerations

### API Authentication
All API endpoints (except health and webhooks) require the `X-API-Key` header.

### Webhook Security
- Razorpay webhooks are verified using HMAC-SHA256 signatures
- Blockchain webhooks should be sent from trusted sources only

### Secrets Management
- Never commit `.env` files
- Use secrets management in production (AWS Secrets Manager, Vault, etc.)
- Rotate API keys periodically

### Rate Limiting
Default: 100 requests/second with burst of 200. Configure via environment variables.

## Production Deployment

### Docker Production

```bash
cd docker
docker-compose --profile production up -d
```

### Kubernetes

See `k8s/` directory for Kubernetes manifests (if available).

### Recommended Architecture

```
                    ┌─────────────┐
                    │   Nginx     │
                    │ (SSL/LB)    │
                    └─────┬───────┘
                          │
          ┌───────────────┼───────────────┐
          │               │               │
    ┌─────▼─────┐   ┌─────▼─────┐   ┌─────▼─────┐
    │ API Pod 1 │   │ API Pod 2 │   │ API Pod N │
    └─────┬─────┘   └─────┬─────┘   └─────┬─────┘
          │               │               │
          └───────────────┼───────────────┘
                          │
                    ┌─────▼─────┐
                    │PostgreSQL │
                    │ (Primary) │
                    └───────────┘
```

### Monitoring

- Health endpoint: `/health`
- Status endpoint: `/api/v1/status`
- Structured JSON logging for log aggregation
- Prometheus metrics (coming soon)

## Development

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration
```

### Code Style

```bash
# Format code
cargo fmt

# Lint
cargo clippy
```

### Database Migrations

```bash
# Create new migration
sqlx migrate add <name>

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

## Troubleshooting

### Database Connection Issues
- Verify `DATABASE_URL` format
- Check PostgreSQL is running
- Verify network connectivity

### Razorpay Integration
- Use test keys (`rzp_test_`) in development
- Verify webhook secret matches

### Blockchain RPC
- Check RPC endpoint is accessible
- Verify chain ID matches network

## License

MIT License

## Support

For issues and feature requests, please open a GitHub issue.
