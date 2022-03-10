mod config {
  pub use ::config::ConfigError;
  use serde::Deserialize;

  #[derive(Deserialize)]
  pub struct Config {
    pub server_addr: String,
    pub pg: deadpool_postgres::Config,
  }

  impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
      let mut cfg = ::config::Config::new();
      cfg.merge(::config::Environment::new())?;
      cfg.try_into()
    }
  }
}

mod models {
  use serde::{Deserialize, Serialize};
  use tokio_pg_mapper_derive::PostgresMapper;

  #[derive(Deserialize, Serialize)]
  pub struct Ping {
    pub message: String,
  }

  #[derive(Deserialize, Serialize)]
  pub struct Filter {
    pub condition: String,
  }

  #[derive(Deserialize, PostgresMapper, Serialize)]
  #[pg_mapper(table = "users")]
  // singular 'user' is a keyword..
  pub struct User {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub username: String,
  }
}

mod errors {
  use actix_web::{HttpResponse, ResponseError};
  use deadpool_postgres::PoolError;
  use derive_more::{Display, From};
  use tokio_pg_mapper::Error as PGMError;
  use tokio_postgres::error::Error as PGError;

  #[derive(Display, From, Debug)]
  pub enum MyError {
    NotFound,
    PGError(PGError),
    PGMError(PGMError),
    PoolError(PoolError),
  }
  impl std::error::Error for MyError {}

  impl ResponseError for MyError {
    fn error_response(&self) -> HttpResponse {
      match *self {
        MyError::NotFound => HttpResponse::NotFound().finish(),
        MyError::PoolError(ref err) => HttpResponse::InternalServerError().body(err.to_string()),
        _ => HttpResponse::InternalServerError().finish(),
      }
    }
  }
}

mod db {
  use crate::{
    errors::MyError,
    models::{Filter, User},
  };
  use deadpool_postgres::Client;
  use tokio_pg_mapper::FromTokioPostgresRow;

  pub async fn add_user(client: &Client, user_info: User) -> Result<User, MyError> {
    let _stmt = include_str!("../sql/add_user.sql");
    let _stmt = _stmt.replace("$table_fields", &User::sql_table_fields());
    let stmt = client.prepare(&_stmt).await.unwrap();

    client
      .query(
        &stmt,
        &[
          &user_info.email,
          &user_info.first_name,
          &user_info.last_name,
          &user_info.username,
        ],
      )
      .await?
      .iter()
      .map(|row| User::from_row_ref(row).unwrap())
      .collect::<Vec<User>>()
      .pop()
      // more applicable for SELECTs
      .ok_or(MyError::NotFound)
  }

  pub async fn get_users(client: &Client, filter: Option<Filter>) -> Result<Vec<User>, MyError> {
    let _stmt = include_str!("../sql/get_users.sql");
    let mut _stmt = _stmt.replace("$table_fields", &User::sql_table_fields());
    match filter {
      Some(value) => _stmt = _stmt.replace("$where", format!("WHERE {}", value.condition).as_str()),
      None => (),
    }

    let stmt = client.prepare(&_stmt).await.unwrap();

    let res = client
      .query(&stmt, &[])
      .await?
      .iter()
      .map(|row| User::from_row_ref(row).unwrap())
      .collect::<Vec<User>>();
    // manually wrap the Vec<User> that comes from collect() in a Result::Ok
    Ok(res)
  }
}

mod handlers {
  use crate::{
    db,
    errors::MyError,
    models::{Filter, Ping, User},
  };
  use actix_web::{web, Error, HttpResponse};
  use deadpool_postgres::{Client, Pool};

  pub async fn ping() -> HttpResponse {
    HttpResponse::Ok().json(Ping {
      message: String::from("pong"),
    })
  }

  pub async fn add_user(
    user: web::Json<User>,
    db_pool: web::Data<Pool>,
  ) -> Result<HttpResponse, Error> {
    let user_info: User = user.into_inner();

    let client: Client = db_pool.get().await.map_err(MyError::PoolError)?;

    let new_user = db::add_user(&client, user_info).await?;

    Ok(HttpResponse::Ok().json(new_user))
  }

  pub async fn get_users(
    filter: web::Json<Option<Filter>>,
    db_pool: web::Data<Pool>,
  ) -> Result<HttpResponse, Error> {
    let filter_info: Option<Filter> = filter.into_inner();

    let client: Client = db_pool.get().await.map_err(MyError::PoolError)?;

    let users = db::get_users(&client, filter_info).await?;

    Ok(HttpResponse::Ok().json(users))
  }
}

use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use handlers::{add_user, get_users, ping};
use tokio_postgres::NoTls;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  dotenv().ok();

  let config = crate::config::Config::from_env().unwrap();
  let pool = config.pg.create_pool(None, NoTls).unwrap();

  let server = HttpServer::new(move || {
    App::new()
      .app_data(web::Data::new(pool.clone()))
      .service(web::resource("/ping").route(web::get().to(ping)))
      .service(
        web::resource("/users")
          .route(web::post().to(add_user))
          .route(web::get().to(get_users)),
      )
  })
  .bind(config.server_addr.clone())?
  .run();

  println!("Server running at http://{}/", config.server_addr);

  server.await
}
