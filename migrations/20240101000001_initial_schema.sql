-- Payment Gateway Initial Schema

-- Custom types
CREATE TYPE payment_status AS ENUM (
    'pending', 'processing', 'completed', 'failed', 'cancelled', 'refunded', 'expired'
);

CREATE TYPE payment_method AS ENUM (
    'card', 'upi', 'net_banking', 'wallet', 'emi',
    'ethereum', 'polygon', 'bsc', 'arbitrum', 'solana', 'lightning'
);

CREATE TYPE currency_type AS ENUM (
    'INR', 'USD', 'EUR', 'ETH', 'MATIC', 'BNB', 'SOL', 'BTC', 'USDT', 'USDC'
);

CREATE TYPE transaction_type AS ENUM (
    'payment', 'refund', 'transfer', 'withdrawal'
);

CREATE TYPE transaction_status AS ENUM (
    'pending', 'confirming', 'confirmed', 'failed', 'cancelled'
);

CREATE TYPE chain_type AS ENUM (
    'ethereum', 'polygon', 'bsc', 'arbitrum', 'solana', 'bitcoin'
);

CREATE TYPE webhook_source AS ENUM (
    'razorpay', 'blockchain', 'lightning', 'internal'
);

CREATE TYPE webhook_status AS ENUM (
    'received', 'processing', 'processed', 'failed', 'ignored'
);

-- Payments table
CREATE TABLE payments (
    id UUID PRIMARY KEY,
    external_id VARCHAR(255),
    order_id VARCHAR(255),
    amount BIGINT NOT NULL,
    currency currency_type NOT NULL,
    status payment_status NOT NULL DEFAULT 'pending',
    method payment_method NOT NULL,
    description TEXT,
    customer_email VARCHAR(255),
    customer_phone VARCHAR(50),
    metadata JSONB,

    -- Razorpay specific
    razorpay_payment_id VARCHAR(255),
    razorpay_order_id VARCHAR(255),
    razorpay_signature VARCHAR(500),

    -- Crypto specific
    crypto_tx_hash VARCHAR(255),
    crypto_from_address VARCHAR(255),
    crypto_to_address VARCHAR(255),
    crypto_chain VARCHAR(50),

    -- Lightning specific
    lightning_invoice TEXT,
    lightning_payment_hash VARCHAR(255),

    expires_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Transactions table
CREATE TABLE transactions (
    id UUID PRIMARY KEY,
    payment_id UUID NOT NULL REFERENCES payments(id) ON DELETE CASCADE,
    tx_type transaction_type NOT NULL,
    status transaction_status NOT NULL DEFAULT 'pending',
    amount BIGINT NOT NULL,
    fee BIGINT,
    currency VARCHAR(20) NOT NULL,
    tx_hash VARCHAR(255),
    block_number BIGINT,
    confirmations INTEGER NOT NULL DEFAULT 0,
    required_confirmations INTEGER NOT NULL DEFAULT 1,
    from_address VARCHAR(255),
    to_address VARCHAR(255),
    chain VARCHAR(50),
    raw_data JSONB,
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Crypto addresses table
CREATE TABLE crypto_addresses (
    id UUID PRIMARY KEY,
    payment_id UUID REFERENCES payments(id) ON DELETE SET NULL,
    address VARCHAR(255) NOT NULL,
    chain chain_type NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    label VARCHAR(255),
    expected_amount BIGINT,
    received_amount BIGINT,
    token_address VARCHAR(255),
    last_checked_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    UNIQUE(address, chain)
);

-- Webhook events table
CREATE TABLE webhook_events (
    id UUID PRIMARY KEY,
    source webhook_source NOT NULL,
    event_type VARCHAR(255) NOT NULL,
    event_id VARCHAR(255),
    payment_id UUID REFERENCES payments(id) ON DELETE SET NULL,
    status webhook_status NOT NULL DEFAULT 'received',
    payload JSONB NOT NULL,
    headers JSONB,
    signature VARCHAR(500),
    signature_verified BOOLEAN NOT NULL DEFAULT false,
    error_message TEXT,
    processed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- API keys table
CREATE TABLE api_keys (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    key_hash VARCHAR(255) NOT NULL UNIQUE,
    key_prefix VARCHAR(20) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    permissions JSONB,
    rate_limit INTEGER,
    last_used_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_payments_status ON payments(status);
CREATE INDEX idx_payments_method ON payments(method);
CREATE INDEX idx_payments_razorpay_order_id ON payments(razorpay_order_id);
CREATE INDEX idx_payments_crypto_to_address ON payments(crypto_to_address);
CREATE INDEX idx_payments_created_at ON payments(created_at DESC);

CREATE INDEX idx_transactions_payment_id ON transactions(payment_id);
CREATE INDEX idx_transactions_tx_hash ON transactions(tx_hash);
CREATE INDEX idx_transactions_status ON transactions(status);

CREATE INDEX idx_crypto_addresses_address_chain ON crypto_addresses(address, chain);
CREATE INDEX idx_crypto_addresses_payment_id ON crypto_addresses(payment_id);
CREATE INDEX idx_crypto_addresses_active ON crypto_addresses(is_active) WHERE is_active = true;

CREATE INDEX idx_webhook_events_source_event_id ON webhook_events(source, event_id);
CREATE INDEX idx_webhook_events_payment_id ON webhook_events(payment_id);
CREATE INDEX idx_webhook_events_status ON webhook_events(status);

CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_active ON api_keys(is_active) WHERE is_active = true;

-- Updated at trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply updated_at triggers
CREATE TRIGGER update_payments_updated_at
    BEFORE UPDATE ON payments
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_transactions_updated_at
    BEFORE UPDATE ON transactions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_crypto_addresses_updated_at
    BEFORE UPDATE ON crypto_addresses
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_api_keys_updated_at
    BEFORE UPDATE ON api_keys
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
