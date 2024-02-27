use chrono::{NaiveDateTime, Utc};
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, prelude::FromRow, PgPool};

use crate::{CreateTransaction, TransactionKind};

pub struct PgRepository {
  pool: PgPool,
}

#[derive(FromRow)]
pub struct PgClient {
  pub id: i32,
  pub limit: i32,
  pub balance: i32,
}

#[derive(Debug, FromRow)]
pub struct PgTransaction {
  pub amount: i32,
  pub kind: String,
  pub description: String,
  pub created_at: NaiveDateTime,
}

impl PgRepository {
  pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
    let pool = PgPoolOptions::new()
      .max_connections(64)
      .connect(url)
      .await?;

    Ok(Self { pool })
  }

  pub async fn extract(&self, client_id: i32) -> Result<serde_json::Value, sqlx::Error> {
    let client: Option<PgClient> = sqlx::query_as(
      r#"
      SELECT "id", "limit", "balance"
      FROM "clients"
      WHERE "id" = $1
      "#,
    )
    .bind(client_id)
    .fetch_optional(&self.pool)
    .await
    .unwrap();

    if let Some(client) = client {
      let client_as_json = json!({
        "total": client.balance,
        "limite": client.limit,
        "data_extrato": Utc::now(),
      });

      let transactions: Vec<PgTransaction> = sqlx::query_as(
        r#"
        SELECT "amount", "kind", "description", "created_at"
        FROM "transactions"
        WHERE "client_id" = $1
        ORDER BY "created_at" DESC
        LIMIT 10
        "#,
      )
      .bind(client_id)
      .fetch_all(&self.pool)
      .await
      .unwrap();

      let mut transactions_as_json = Vec::new();

      for transaction in transactions.iter() {
        transactions_as_json.push(json!({
          "valor": transaction.amount,
          "tipo": transaction.kind,
          "descricao": &transaction.description,
          "realizada_em": transaction.created_at,
        }));
      }

      return Ok(json!({
        "saldo": client_as_json,
        "ultimas_transacoes": transactions_as_json,
      }));
    }

    Err(sqlx::Error::RowNotFound)
  }

  pub async fn transact(
    &self,
    client_id: i32,
    create_transaction: &CreateTransaction,
  ) -> Result<PgClient, sqlx::Error> {
    let mut transaction = self.pool.begin().await.unwrap();

    let client: Option<PgClient> = sqlx::query_as(
      r#"
      SELECT "id", "limit", "balance"
      FROM "clients"
      WHERE "id" = $1
      FOR UPDATE
      "#,
    )
    .bind(client_id)
    .fetch_optional(&mut *transaction)
    .await
    .unwrap();

    if let Some(mut client) = client {
      let transaction_kind = match create_transaction.tipo {
        TransactionKind::Credit => {
          client.balance += create_transaction.valor;
          "c"
        }
        TransactionKind::Debit => {
          if client.balance + client.limit < create_transaction.valor {
            return Err(sqlx::Error::WorkerCrashed);
          }
          client.balance -= create_transaction.valor;
          "d"
        }
      };

      sqlx::query(
        r#"
        UPDATE "clients"
        SET "balance" = $1
        WHERE "id" = $2
        "#,
      )
      .bind(client.balance)
      .bind(client.id)
      .execute(&mut *transaction)
      .await
      .unwrap();

      sqlx::query(
        r#"
        INSERT INTO "transactions" (
          "kind", "amount", "description", "client_id", "created_at"
        )
        VALUES (
          $1, $2, $3, $4, $5
        )
        "#,
      )
      .bind(transaction_kind)
      .bind(create_transaction.valor)
      .bind(&create_transaction.descricao.0)
      .bind(client.id)
      .bind(Utc::now().naive_utc())
      .execute(&mut *transaction)
      .await
      .unwrap();

      transaction.commit().await.unwrap();

      return Ok(client);
    }

    Err(sqlx::Error::RowNotFound)
  }
}
