# NOTES

- [NOTES](#notes)
  - [Pre Requisites](#pre-requisites)
    - [Tooling](#tooling)
    - [Postgres](#postgres)
  - [Bootstrap project](#bootstrap-project)
  - [Convert a collection to a Result](#convert-a-collection-to-a-result)
  - [Update using an existing tokio_postgres::Config](#update-using-an-existing-tokio_postgresconfig)

## Pre Requisites

### Tooling

- [Open Source SQL Editor and Database Manager](https://www.beekeeperstudio.io/)

### Postgres

> update: used docker postgres

- [SDB:PostgreSQL - openSUSE Wiki](https://en.opensuse.org/SDB:PostgreSQL)

install client for psql, createuser, createdb etc

```shell
$ sudo zypper in postgresql
$ psql --version
psql (PostgreSQL) 14.1
```

## Bootstrap project

opted to enter inside container

```shell
$ docker exec -it db bash
# create user
$ createuser -P test_user
Enter password for new role: 
Enter it again: 
createuser: error: could not connect to database template1: FATAL:  role "root" does not exist
```

- [Createuser: could not connect to database postgres: FATAL: role &quot;tom&quot; does not exist](https://stackoverflow.com/questions/16973018/createuser-could-not-connect-to-database-postgres-fatal-role-tom-does-not-e)

```shell
# now it works
$ createuser -P test_user -U postgres
Enter password for new role: 
Enter it again: 

# create db
$ createdb -O test_user testing_db -U postgres

# exit container
exit

# copy scheme
$ docker cp sql/schema.sql db:/tmp
# enter container
$ docker exec -it db bash

$ psql -f /tmp/schema.sql testing_db -U postgres
psql:/tmp/schema.sql:1: NOTICE:  schema "testing" does not exist, skipping
DROP SCHEMA
CREATE SCHEMA
CREATE TABLE
```

> This step can be repeated and clears the database as it drops and recreates the schema testing which is used within the database.

use pgAdmin to grant user on **testing schema**

![image](2022-03-09-22-47-42.png)

```sql
ALTER TABLE IF EXISTS default_schema.users OWNER to postgres;
GRANT ALL ON TABLE default_schema.users TO postgres;
GRANT ALL ON TABLE default_schema.users TO test_user;
```

```shell
# create .env file:
$ code .env
```

```shell
SERVER_ADDR=127.0.0.1:8080
PG.USER=test_user
PG.PASSWORD=testing
PG.HOST=127.0.0.1
PG.PORT=5432
PG.DBNAME=testing_db
PG.POOL.MAX_SIZE=16
```

```shell
# run the server:
$ cargo run
```

## Convert a collection to a Result

- [How do you convert a collection to a Result in rust and actix to return data from postgres?](https://stackoverflow.com/questions/71189663/how-do-you-convert-a-collection-to-a-result-in-rust-and-actix-to-return-data-fro)

```rust
  // more applicable for SELECTs
  .ok_or(MyError::NotFound)
}
```

## Update using an existing tokio_postgres::Config

in starter project, now we don't use bellow dependencies, we opted for a manually config based on env variables only

```shell
# config
config = "0.11.0"
dotenv = "0.15.0"
```

- [Example using an existing tokio_postgres::Config object](https://docs.rs/deadpool-postgres/latest/deadpool_postgres/#example-using-an-existing-tokio_postgresconfig-object)

```rust
// config postgres
let mut pg_config = tokio_postgres::Config::new();
pg_config.user(env::var("PG_USER").unwrap_or(DEFAULT_PG_USER.to_string()).as_str());
pg_config.password(env::var("PG_PASSWORD").unwrap_or(DEFAULT_PG_PASSWORD.to_string()).as_str());
pg_config.host(env::var("PG_HOST").unwrap_or(DEFAULT_PG_HOST.to_string()).as_str());
pg_config.port(env::var("PG_PORT").unwrap_or(DEFAULT_PG_PORT.to_string()).parse::<i16>().unwrap() as u16);
pg_config.dbname(env::var("PG_DBNAME").unwrap_or(DEFAULT_PG_DBNAME.to_string()).as_str());
let mgr_config = ManagerConfig {
  recycling_method: RecyclingMethod::Fast,
};
let mgr = Manager::from_config(pg_config, NoTls, mgr_config);
let pool = Pool::builder(mgr)
  .max_size(env::var("PG_POOL_MAX_SIZE").unwrap_or(DEFAULT_PG_POOL_MAX_SIZE.to_string()).parse::<usize>().unwrap())
  .build()
  .unwrap();
// If you want your application to crash on startup if no database connection can be established just call pool.get().await right after creating the pool.
match pool.get().await {
  Ok(_) => info!("database connection can be established"),
  Err(e) => error!("{:?}", e),
}
```
