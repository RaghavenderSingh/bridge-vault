use crate::error::{RelayerError, Result};
use crate::types::{Chain, RelayerTransaction, TransactionStatus};
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use tracing::{info, warn};

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str, max_connections: u32) -> Result<Self> {
        info!("Connecting to database: {}", database_url);

        let options = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| RelayerError::DatabaseError(sqlx::Error::Configuration(Box::new(e))))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect_with(options)
            .await?;

        let db = Database { pool };
        db.run_migrations().await?;

        Ok(db)
    }

    async fn run_migrations(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS relayer_transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                nonce INTEGER NOT NULL UNIQUE,
                from_chain TEXT NOT NULL,
                to_chain TEXT NOT NULL,
                from_tx_hash TEXT NOT NULL UNIQUE,
                to_tx_hash TEXT,
                sender TEXT NOT NULL,
                recipient TEXT NOT NULL,
                amount INTEGER NOT NULL,
                status TEXT NOT NULL,
                signatures TEXT,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_nonce ON relayer_transactions(nonce);
            CREATE INDEX IF NOT EXISTS idx_status ON relayer_transactions(status);
            CREATE INDEX IF NOT EXISTS idx_from_tx_hash ON relayer_transactions(from_tx_hash);
            CREATE INDEX IF NOT EXISTS idx_to_tx_hash ON relayer_transactions(to_tx_hash);
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn create_transaction(
        &self,
        nonce: u64,
        from_chain: Chain,
        to_chain: Chain,
        from_tx_hash: &str,
        sender: &str,
        recipient: &str,
        amount: u64,
    ) -> Result<i64> {
        let now = Utc::now();
        let result = sqlx::query(
            r#"
            INSERT INTO relayer_transactions
            (nonce, from_chain, to_chain, from_tx_hash, sender, recipient, amount, status, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(nonce as i64)
        .bind(from_chain)
        .bind(to_chain)
        .bind(from_tx_hash)
        .bind(sender)
        .bind(recipient)
        .bind(amount as i64)
        .bind(TransactionStatus::Pending)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn get_transaction_by_nonce(&self, nonce: u64) -> Result<Option<RelayerTransaction>> {
        let tx = sqlx::query_as::<_, RelayerTransaction>(
            "SELECT * FROM relayer_transactions WHERE nonce = ?",
        )
        .bind(nonce as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(tx)
    }

    pub async fn get_transaction_by_hash(&self, tx_hash: &str) -> Result<Option<RelayerTransaction>> {
        let tx = sqlx::query_as::<_, RelayerTransaction>(
            "SELECT * FROM relayer_transactions WHERE from_tx_hash = ?",
        )
        .bind(tx_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(tx)
    }

    pub async fn update_transaction_status(
        &self,
        id: i64,
        status: TransactionStatus,
        to_tx_hash: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE relayer_transactions
            SET status = ?, to_tx_hash = ?, error_message = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(to_tx_hash)
        .bind(error_message)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_signatures(&self, id: i64, signatures: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE relayer_transactions
            SET signatures = ?, status = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(signatures)
        .bind(TransactionStatus::SignaturesCollected)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_pending_transactions(&self) -> Result<Vec<RelayerTransaction>> {
        let txs = sqlx::query_as::<_, RelayerTransaction>(
            "SELECT * FROM relayer_transactions WHERE status = ? OR status = ? ORDER BY created_at ASC",
        )
        .bind(TransactionStatus::Pending)
        .bind(TransactionStatus::SignaturesCollected)
        .fetch_all(&self.pool)
        .await?;

        Ok(txs)
    }

    pub async fn get_transactions_by_status(
        &self,
        status: TransactionStatus,
    ) -> Result<Vec<RelayerTransaction>> {
        let txs = sqlx::query_as::<_, RelayerTransaction>(
            "SELECT * FROM relayer_transactions WHERE status = ? ORDER BY created_at DESC LIMIT 100",
        )
        .bind(status)
        .fetch_all(&self.pool)
        .await?;

        Ok(txs)
    }

    pub async fn is_nonce_processed(&self, nonce: u64) -> Result<bool> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM relayer_transactions WHERE nonce = ?")
            .bind(nonce as i64)
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0 > 0)
    }

    pub async fn get_stats(&self) -> Result<TransactionStats> {
        let stats = sqlx::query_as::<_, TransactionStats>(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN status = 'Pending' THEN 1 ELSE 0 END) as pending,
                SUM(CASE WHEN status = 'SignaturesCollected' THEN 1 ELSE 0 END) as signatures_collected,
                SUM(CASE WHEN status = 'Submitted' THEN 1 ELSE 0 END) as submitted,
                SUM(CASE WHEN status = 'Confirmed' THEN 1 ELSE 0 END) as confirmed,
                SUM(CASE WHEN status = 'Failed' THEN 1 ELSE 0 END) as failed
            FROM relayer_transactions
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct TransactionStats {
    pub total: i64,
    pub pending: i64,
    pub signatures_collected: i64,
    pub submitted: i64,
    pub confirmed: i64,
    pub failed: i64,
}
