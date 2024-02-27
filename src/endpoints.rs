use std::sync::Arc;

use actix_web::{
  get, post,
  web::{Data, Json, Path},
  HttpResponse, Responder,
};
use serde_json::json;

use crate::{AppState, CreateTransaction};

//bank statement
#[get("/clientes/{id}/extrato")]
pub async fn view_extract(path: Path<i32>, state: Data<Arc<AppState>>) -> impl Responder {
  let client_id = path.into_inner();

  let extract = state.pg_repository.extract(client_id).await;

  match extract {
    Ok(extract) => HttpResponse::Ok().json(extract),
    Err(_) => HttpResponse::NotFound().finish(),
  }
}

#[post("/clientes/{id}/transacoes")]
pub async fn create_transaction(
  path: Path<i32>,
  state: Data<Arc<AppState>>,
  Json(create_transaction): Json<CreateTransaction>,
) -> impl Responder {
  let client_id = path.into_inner();

  let transact = state
    .pg_repository
    .transact(client_id, &create_transaction)
    .await;

  match transact {
    Ok(client) => HttpResponse::Ok().json(json!({
      "limite": client.limit,
      "saldo": client.balance,
    })),
    Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().finish(),
    Err(_) => HttpResponse::UnprocessableEntity().finish(),
  }
}
