use crate::db::{Database, TransactionStats};
use crate::types::{RelayerTransaction, TransactionStatus};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// API state shared across handlers
#[derive(Clone)]
pub struct ApiState {
    pub db: Database,
}

/// Transaction response matching CLI's TxResponse
#[derive(Debug, Serialize, Deserialize)]
pub struct TxResponse {
    pub nonce: u64,
    pub from_chain: String,
    pub to_chain: String,
    pub from_tx_hash: String,
    pub to_tx_hash: Option<String>,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub status: String,
    pub error_message: Option<String>,
}

impl From<RelayerTransaction> for TxResponse {
    fn from(tx: RelayerTransaction) -> Self {
        Self {
            nonce: tx.nonce as u64,
            from_chain: tx.from_chain.to_string(),
            to_chain: tx.to_chain.to_string(),
            from_tx_hash: tx.from_tx_hash,
            to_tx_hash: tx.to_tx_hash,
            sender: tx.sender,
            recipient: tx.recipient,
            amount: tx.amount as u64,
            status: tx.status.to_string().to_lowercase(),
            error_message: tx.error_message,
        }
    }
}

/// Transaction summary for list view
#[derive(Debug, Serialize, Deserialize)]
pub struct TxSummary {
    pub nonce: u64,
    pub from_chain: String,
    pub to_chain: String,
    pub amount: u64,
    pub status: String,
}

impl From<&RelayerTransaction> for TxSummary {
    fn from(tx: &RelayerTransaction) -> Self {
        Self {
            nonce: tx.nonce as u64,
            from_chain: tx.from_chain.to_string(),
            to_chain: tx.to_chain.to_string(),
            amount: tx.amount as u64,
            status: tx.status.to_string().to_lowercase(),
        }
    }
}

/// List response matching CLI's TxsListResponse
#[derive(Debug, Serialize, Deserialize)]
pub struct TxsListResponse {
    pub transactions: Vec<TxSummary>,
    pub total: usize,
}

/// Stats response
#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub total: i64,
    pub pending: i64,
    pub signatures_collected: i64,
    pub submitted: i64,
    pub confirmed: i64,
    pub failed: i64,
}

impl From<TransactionStats> for StatsResponse {
    fn from(stats: TransactionStats) -> Self {
        Self {
            total: stats.total,
            pending: stats.pending,
            signatures_collected: stats.signatures_collected,
            submitted: stats.submitted,
            confirmed: stats.confirmed,
            failed: stats.failed,
        }
    }
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Query parameters for listing transactions
#[derive(Debug, Deserialize)]
pub struct ListTransactionsQuery {
    pub user: Option<String>,
}

/// Get transaction by nonce
async fn get_transaction(
    State(state): State<Arc<ApiState>>,
    Path(nonce): Path<u64>,
) -> Result<Json<TxResponse>, StatusCode> {
    info!("API: Getting transaction for nonce {}", nonce);

    match state.db.get_transaction_by_nonce(nonce).await {
        Ok(Some(tx)) => Ok(Json(tx.into())),
        Ok(None) => {
            warn!("Transaction not found for nonce {}", nonce);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            warn!("Database error fetching transaction {}: {}", nonce, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List transactions with optional user filter
async fn list_transactions(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<ListTransactionsQuery>,
) -> Result<Json<TxsListResponse>, StatusCode> {
    info!("API: Listing transactions, user filter: {:?}", query.user);

    match state.db.list_transactions(query.user.as_deref()).await {
        Ok(txs) => {
            let total = txs.len();
            let transactions: Vec<TxSummary> = txs.iter().map(|tx| tx.into()).collect();
            Ok(Json(TxsListResponse {
                transactions,
                total,
            }))
        }
        Err(e) => {
            warn!("Database error listing transactions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get transaction statistics
async fn get_stats(State(state): State<Arc<ApiState>>) -> Result<Json<StatsResponse>, StatusCode> {
    info!("API: Getting stats");

    match state.db.get_stats().await {
        Ok(stats) => Ok(Json(stats.into())),
        Err(e) => {
            warn!("Database error fetching stats: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Health check endpoint
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Create the API router
pub fn create_router(state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/tx/:nonce", get(get_transaction))
        .route("/txs", get(list_transactions))
        .route("/stats", get(get_stats))
        .with_state(state)
}

/// Start the HTTP API server
pub async fn start_server(
    state: ApiState,
    port: u16,
) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    let app = create_router(Arc::new(state));

    info!("HTTP API server listening on http://{}", addr);
    info!("Available endpoints:");
    info!("  GET /health     - Health check");
    info!("  GET /tx/:nonce  - Get transaction by nonce");
    info!("  GET /txs        - List transactions (optional: ?user=<address>)");
    info!("  GET /stats      - Transaction statistics");

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::TransactionStats;
    use crate::types::{Chain, TransactionStatus};
    use chrono::Utc;

    /// Test that TxResponse serializes to the expected JSON format
    #[test]
    fn test_tx_response_serialization() {
        let tx = RelayerTransaction {
            id: 1,
            nonce: 42,
            from_chain: Chain::Solana,
            to_chain: Chain::Ethereum,
            from_tx_hash: "abc123".to_string(),
            to_tx_hash: Some("def456".to_string()),
            sender: "sender_addr".to_string(),
            recipient: "recipient_addr".to_string(),
            amount: 1000000000,
            status: TransactionStatus::Confirmed,
            signatures: None,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let response: TxResponse = tx.into();
        let json = serde_json::to_string(&response).unwrap();

        // Verify lowercase status
        assert!(json.contains("\"status\":\"confirmed\""), "Status should be lowercase: {}", json);
        assert!(json.contains("\"nonce\":42"), "JSON: {}", json);
        assert!(json.contains("\"amount\":1000000000"), "JSON: {}", json);
    }

    /// Test that all status variants serialize to lowercase
    #[test]
    fn test_all_status_variants_serialize_correctly() {
        let test_cases = vec![
            (TransactionStatus::Pending, "pending"),
            (TransactionStatus::SignaturesCollected, "signaturescollected"),
            (TransactionStatus::Submitted, "submitted"),
            (TransactionStatus::Confirmed, "confirmed"),
            (TransactionStatus::Failed, "failed"),
        ];

        for (status, expected) in test_cases {
            let tx = RelayerTransaction {
                id: 1,
                nonce: 1,
                from_chain: Chain::Solana,
                to_chain: Chain::Ethereum,
                from_tx_hash: "test".to_string(),
                to_tx_hash: None,
                sender: "sender".to_string(),
                recipient: "recipient".to_string(),
                amount: 100,
                status,
                signatures: None,
                error_message: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            let response: TxResponse = tx.into();
            let json = serde_json::to_string(&response).unwrap();
            let expected_str = format!("\"status\":\"{}\"", expected);
            assert!(json.contains(&expected_str), "Expected {} in {}", expected_str, json);
        }
    }

    /// Test TxSummary serialization
    #[test]
    fn test_tx_summary_serialization() {
        let tx = RelayerTransaction {
            id: 1,
            nonce: 42,
            from_chain: Chain::Solana,
            to_chain: Chain::Ethereum,
            from_tx_hash: "abc".to_string(),
            to_tx_hash: None,
            sender: "sender".to_string(),
            recipient: "recipient".to_string(),
            amount: 1000,
            status: TransactionStatus::Pending,
            signatures: None,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let summary: TxSummary = (&tx).into();
        let json = serde_json::to_string(&summary).unwrap();

        assert!(json.contains("\"nonce\":42"), "JSON: {}", json);
        assert!(json.contains("\"status\":\"pending\""), "JSON: {}", json);
        assert!(json.contains("\"amount\":1000"), "JSON: {}", json);
    }

    /// Test StatsResponse serialization
    #[test]
    fn test_stats_response_serialization() {
        let stats = TransactionStats {
            total: 100,
            pending: 10,
            signatures_collected: 5,
            submitted: 20,
            confirmed: 60,
            failed: 5,
        };

        let response: StatsResponse = stats.into();
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"total\":100"), "JSON: {}", json);
        assert!(json.contains("\"pending\":10"), "JSON: {}", json);
        assert!(json.contains("\"confirmed\":60"), "JSON: {}", json);
    }
}
