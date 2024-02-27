use std::{env, sync::Arc};

use actix_web::{
  error,
  web::{Data, JsonConfig},
  App, HttpResponse, HttpServer,
};
use endpoints::{create_transaction, view_extract};
use persistence::PgRepository;
use serde::{Deserialize, Serialize};

pub mod endpoints;
pub mod persistence;

pub struct AppState {
  pg_repository: PgRepository,
}

#[derive(Deserialize)]
pub struct CreateTransaction {
  pub valor: i32,
  pub tipo: TransactionKind,
  pub descricao: TransactionDescription,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum TransactionKind {
  #[serde(rename = "c")]
  Credit,
  #[serde(rename = "d")]
  Debit,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(try_from = "String")]
pub struct TransactionDescription(pub String);

impl TryFrom<String> for TransactionDescription {
  type Error = &'static str;

  fn try_from(description: String) -> Result<Self, Self::Error> {
    if description.is_empty() || description.len() > 10 {
      return Err("description not allowed");
    }

    Ok(Self(description))
  }
}

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
  let port = env::var("PORT")
    .ok()
    .and_then(|port| port.parse::<u16>().ok())
    .unwrap_or(3000);

  let url = env::var("DATABASE_URL")
    .unwrap_or(String::from("postgres://rinha:rinha@localhost:5432/rinha"));

  let pg_repository = PgRepository::new(&url).await.unwrap();

  let state = Arc::new(AppState {
    pg_repository,
  });

  HttpServer::new(move || {
    App::new()
      .app_data(JsonConfig::default().error_handler(|err, _| {
        error::InternalError::from_response(err, HttpResponse::UnprocessableEntity().finish())
          .into()
      }))
      .app_data(Data::new(state.clone()))
      .service(view_extract)
      .service(create_transaction)
  })
  .bind(format!("0.0.0.0:{port}"))?
  .run()
  .await
}
