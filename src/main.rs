mod constants {
  #[allow(dead_code)]
  pub const APP_NAME: &'static str = "actixweb 4.0 deadpool-postgres starter";
  pub const I18N_RECORD_NOT_FOUND: &'static str = "record not found";
  pub const I18N_CANT_CREATE_RECORD: &'static str = "can't create record";
}
mod config {
  pub use ::config::ConfigError;
  use serde::Deserialize;

  #[derive(Deserialize)]
  pub struct Config {
    pub server_addr: String,
    pub server_cert: String,
    pub server_key: String,
    pub server_keep_alive: u64,
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
  pub struct Message {
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
      None => _stmt = _stmt.replace("$where", ""),
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

  pub async fn delete_user(client: &Client, username: String) -> Result<(), MyError> {
    let _stmt = include_str!("../sql/delete_user.sql");

    let stmt = client.prepare(&_stmt).await.unwrap();

    let _res = client.query(&stmt, &[&username]).await?;
    Ok(())
  }
}

mod handlers {
  use crate::{
    constants::{I18N_CANT_CREATE_RECORD, I18N_RECORD_NOT_FOUND},
    db,
    errors::MyError,
    models::{Filter, Message, User},
  };
  use actix_web::{delete, get, http::{StatusCode, header}, post, web, Error, HttpResponse};
  use deadpool_postgres::{Client, Pool};
  #[allow(unused_imports)]
  use log::{debug, error};

  #[get("/ping")]
  pub async fn ping() -> HttpResponse {
    HttpResponse::Ok().json(Message {
      message: String::from("pong"),
    })
  }

  #[get("/redirect")]
  pub async fn redirect() -> HttpResponse {
    // deprecated
    // HttpResponse::Found()
    //   .header(http::header::LOCATION, "https://kuartzo.com")
    //   .finish()
    // HttpResponse::Found()
    // .append_header("header::ContentType(mime::APPLICATION_JSON)"  )
    // .append_header("http::header::LOCATION.as_str()", "https://kuartzo.com")
    //   .finish()
    // TODO:
    // https://docs.rs/actix-web/latest/actix_web/struct.HttpResponseBuilder.html#method.append_header
    // https://crates.io/crates/mime

    HttpResponse::Found()
      // optional
      .append_header(header::ContentType(mime::TEXT_HTML))
      // .append_header(("X-TEST", "value1"))
      .append_header(("location", "https://google.com"))
      .finish()
    // HttpResponse::Found()
    //   //.append_header(header::ContentType(mime::APPLICATION_JSON))
    //   .append_header(("X-TEST", "value1"))
    //   .append_header(("X-TEST", "value2"))
    //   .finish();
  }

  pub async fn not_found() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::build(StatusCode::NOT_FOUND).json(Message {
      message: String::from("not found"),
    }))
  }

  #[post("/users")]
  pub async fn add_user(
    user: web::Json<User>,
    db_pool: web::Data<Pool>,
  ) -> Result<HttpResponse, Error> {
    let user_info: User = user.into_inner();

    let client: Client = db_pool.get().await.map_err(MyError::PoolError)?;

    match db::add_user(&client, user_info).await {
      Ok(new_user) => Ok(HttpResponse::Ok().json(new_user)),
      Err(_e) => {
        // error!("error {:?}", e);
        Ok(HttpResponse::Conflict().json(Message {
          message: format!("{}", I18N_CANT_CREATE_RECORD),
        }))
      }
    }
  }

  #[get("/users")]
  pub async fn get_users(
    filter: Option<web::Json<Filter>>,
    db_pool: web::Data<Pool>,
  ) -> Result<HttpResponse, Error> {
    let mut filter_info: Option<Filter> = None;
    // override none
    if let Some(value) = filter {
      filter_info = Some(value.into_inner())
    };

    let client: Client = db_pool.get().await.map_err(MyError::PoolError)?;

    let users = db::get_users(&client, filter_info).await?;

    Ok(HttpResponse::Ok().json(users))
  }

  #[get("/users/{username}")]
  pub async fn get_user(
    path: web::Path<String>,
    db_pool: web::Data<Pool>,
  ) -> Result<HttpResponse, Error> {
    let username = path.into_inner();
    let filter_info: Option<Filter> = Some(Filter {
      condition: format!("users.username = '{}'", username),
    });

    let client: Client = db_pool.get().await.map_err(MyError::PoolError)?;

    let user = db::get_users(&client, filter_info).await?;

    if user.len() > 0 {
      Ok(HttpResponse::Ok().json(user.get(0)))
    } else {
      Ok(HttpResponse::NotFound().json(Message {
        message: format!("{}", I18N_RECORD_NOT_FOUND),
      }))
    }
  }

  #[delete("/users/{username}")]
  pub async fn delete_user(
    path: web::Path<String>,
    db_pool: web::Data<Pool>,
  ) -> Result<HttpResponse, Error> {
    let username = path.into_inner();

    let client: Client = db_pool.get().await.map_err(MyError::PoolError)?;

    let filter_info: Option<Filter> = Some(Filter {
      condition: format!("users.username = '{}'", username),
    });
    let user = db::get_users(&client, filter_info).await?;

    if user.len() > 0 {
      db::delete_user(&client, username).await?;
      Ok(HttpResponse::Ok().finish())
    } else {
      Ok(HttpResponse::NotFound().json(Message {
        message: format!("{}", I18N_RECORD_NOT_FOUND),
      }))
    }
  }
}

use std::time::Duration;

use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use handlers::{add_user, delete_user, get_user, get_users, not_found, ping, redirect};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use tokio_postgres::NoTls;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  dotenv().ok();

  let config = crate::config::Config::from_env().unwrap();
  let pool = config.pg.create_pool(None, NoTls).unwrap();

  // load ssl keys
  // to create a self-signed temporary cert for testing:
  // openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 3650 -subj '/CN=localhost'
  let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
  builder
    .set_private_key_file(config.server_key.clone(), SslFiletype::PEM)
    .unwrap();
  builder
    .set_certificate_chain_file(config.server_cert.clone())
    .unwrap();

  let server = HttpServer::new(move || {
    App::new()
      .app_data(web::Data::new(pool.clone()))
      .service(ping)
      .service(redirect)
      .service(
        web::scope("/api")
          .service(add_user)
          .service(get_users)
          .service(get_user)
          .service(delete_user),
      )
      .default_service(web::route().to(not_found))
  })
  .keep_alive(Duration::from_secs(config.server_keep_alive))
  // .bind(config.server_addr.clone())?
  .bind_openssl(config.server_addr.clone(), builder)?
  .run();

  println!("Server running at https://{}/", config.server_addr);

  server.await
}
