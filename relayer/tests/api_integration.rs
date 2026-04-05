use relayer::api::{create_router, ApiState, TxsListResponse, TxResponse, StatsResponse};
use relayer::db::Database;
use relayer::types::{Chain, TransactionStatus};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;

/// Helper function to create a test database and API router
async fn setup_test_app() -> (axum::Router, Database) {
    // Create in-memory database
    let db = Database::new("sqlite::memory:", 1).await.expect("Failed to create test DB");

    // Insert some test transactions
    db.create_transaction(
        1,
        Chain::Solana,
        Chain::Ethereum,
        "sol_tx_hash_1",
        "sol_sender_1",
        "eth_recipient_1",
        1000000000,
    ).await.expect("Failed to create tx 1");

    db.create_transaction(
        2,
        Chain::Ethereum,
        Chain::Solana,
        "eth_tx_hash_2",
        "eth_sender_2",
        "sol_recipient_2",
        2000000000,
    ).await.expect("Failed to create tx 2");

    db.create_transaction(
        3,
        Chain::Solana,
        Chain::Ethereum,
        "sol_tx_hash_3",
        "sol_sender_3",
        "eth_recipient_3",
        3000000000,
    ).await.expect("Failed to create tx 3");

    // Create API state and router
    let state = Arc::new(ApiState { db: db.clone() });
    let app = create_router(state);

    (app, db)
}

#[tokio::test]
async fn test_health_endpoint() {
    let (app, _db) = setup_test_app().await;

    let response = app
        .oneshot(Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "healthy");
    assert!(json["version"].as_str().is_some());
}

#[tokio::test]
async fn test_get_transaction_by_nonce() {
    let (app, _db) = setup_test_app().await;

    let response = app
        .oneshot(Request::builder()
            .uri("/tx/1")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let tx: TxResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(tx.nonce, 1);
    assert_eq!(tx.from_chain, "Solana");
    assert_eq!(tx.to_chain, "Ethereum");
    assert_eq!(tx.amount, 1000000000);
    assert_eq!(tx.status, "pending");
    assert_eq!(tx.sender, "sol_sender_1");
    assert_eq!(tx.recipient, "eth_recipient_1");
}

#[tokio::test]
async fn test_get_transaction_not_found() {
    let (app, _db) = setup_test_app().await;

    let response = app
        .oneshot(Request::builder()
            .uri("/tx/999")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_transactions() {
    let (app, _db) = setup_test_app().await;

    let response = app
        .oneshot(Request::builder()
            .uri("/txs")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let list: TxsListResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(list.total, 3);
    assert_eq!(list.transactions.len(), 3);

    // Verify transactions are in descending order (newest first)
    assert_eq!(list.transactions[0].nonce, 3);
    assert_eq!(list.transactions[1].nonce, 2);
    assert_eq!(list.transactions[2].nonce, 1);
}

#[tokio::test]
async fn test_list_transactions_with_user_filter() {
    let (app, _db) = setup_test_app().await;

    let response = app
        .oneshot(Request::builder()
            .uri("/txs?user=sol_sender_1")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let list: TxsListResponse = serde_json::from_slice(&body).unwrap();

    // Should only return tx with sender=sol_sender_1
    assert_eq!(list.total, 1);
    assert_eq!(list.transactions.len(), 1);
    assert_eq!(list.transactions[0].nonce, 1);
}

#[tokio::test]
async fn test_list_transactions_with_recipient_filter() {
    let (app, _db) = setup_test_app().await;

    let response = app
        .oneshot(Request::builder()
            .uri("/txs?user=eth_recipient_3")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let list: TxsListResponse = serde_json::from_slice(&body).unwrap();

    // Should return tx with recipient=eth_recipient_3
    assert_eq!(list.total, 1);
    assert_eq!(list.transactions[0].nonce, 3);
}

#[tokio::test]
async fn test_get_stats() {
    let (app, db) = setup_test_app().await;

    // Update one transaction to confirmed status
    let tx = db.get_transaction_by_nonce(1).await.unwrap().unwrap();
    db.update_transaction_status(
        tx.id,
        TransactionStatus::Confirmed,
        Some("eth_tx_hash"),
        None,
    ).await.unwrap();

    // Update another to failed
    let tx = db.get_transaction_by_nonce(2).await.unwrap().unwrap();
    db.update_transaction_status(
        tx.id,
        TransactionStatus::Failed,
        None,
        Some("Error message"),
    ).await.unwrap();

    let response = app
        .oneshot(Request::builder()
            .uri("/stats")
            .body(Body::empty())
            .unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let stats: StatsResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(stats.total, 3);
    assert_eq!(stats.pending, 1);  // nonce 3
    assert_eq!(stats.confirmed, 1); // nonce 1
    assert_eq!(stats.failed, 1);    // nonce 2
}

#[tokio::test]
async fn test_transaction_status_serialization() {
    let (app, db) = setup_test_app().await;

    // Update transaction to each status and verify serialization
    let test_cases = vec![
        (TransactionStatus::Pending, "pending"),
        (TransactionStatus::SignaturesCollected, "signaturescollected"),
        (TransactionStatus::Submitted, "submitted"),
        (TransactionStatus::Confirmed, "confirmed"),
        (TransactionStatus::Failed, "failed"),
    ];

    for (status, expected_str) in test_cases {
        let tx = db.get_transaction_by_nonce(1).await.unwrap().unwrap();
        db.update_transaction_status(
            tx.id,
            status,
            None,
            None,
        ).await.unwrap();

        let response = app
            .clone()
            .oneshot(Request::builder()
                .uri("/tx/1")
                .body(Body::empty())
                .unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(
            json["status"].as_str().unwrap(),
            expected_str,
            "Status {} should serialize to {}",
            status,
            expected_str
        );
    }
}
