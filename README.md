# Extended ActixWeb 4.0 - async_pg example

## Extended ActixWeb 4.0

2022-03-12 12:25:11

added crud and other missing pieces on top of [actixweb 4.0 postgres example](https://github.com/actix/examples/tree/master/databases/postgres)

- added crud endpoints
- https/ssl
- health check endpoint
- redirect endpoint

## This example illustrates

- `tokio_postgres`
- use of `tokio_pg_mapper` for postgres data mapping
- `deadpool_postgres` for connection pooling
- `dotenv` + `config` for configuration

## Instructions

1. Create database user

```shell
createuser -P test_user
```

Enter a password of your choice. The following instructions assume you used `testing` as password.

This step is **optional** and you can also use an existing database user for that. Just make sure to replace `test_user` by the database user of your choice in the following steps and change the `.env` file containing the configuration accordingly.

2. Create database

```shell
createdb -O test_user testing_db
```

3. Initialize database

```shell
psql -f sql/schema.sql testing_db
```

This step can be repeated and clears the database as it drops and recreates the schema `testing` which is used within the database.

4. Create `.env` file:

```ini
SERVER_ADDR=127.0.0.1:8080
PG.USER=test_user
PG.PASSWORD=testing
PG.HOST=127.0.0.1
PG.PORT=5432
PG.DBNAME=testing_db
PG.POOL.MAX_SIZE=16
```

5. Run the server:

```shell
cargo run
```

6. Using a different terminal send an HTTP POST request to the running server:

```shell
# ping
$ curl -k -X GET https://127.0.0.1:8443/ping \
  -H 'Content-Type: application/json' \
  | jq

# create user ferreal
$ curl -k -X POST https://127.0.0.1:8443/api/users \
  -H 'Content-Type: application/json' \
  -d '{"email": "ferris@thecrab.com", "first_name": "ferris", "last_name": "crab", "username": "ferreal"}' \
  | jq

# create user rustris
$ curl -k -X POST https://127.0.0.1:8443/api/users \
  -H 'Content-Type: application/json' \
  -d '{"email": "rustris@thecrab.com", "first_name": "rustris", "last_name": "crab", "username": "rustris"}' \
  | jq

# get all users
$ curl -k -X GET https://127.0.0.1:8443/api/users \
  -H 'Content-Type: application/json' \
  | jq

# get users / filtered
$ curl -k -X GET https://127.0.0.1:8443/api/users \
  -H 'Content-Type: application/json' \
  -d "{\"condition\": \"users.email = 'rustris@thecrab.com'\"}" \
  | jq

# get user
$ curl -k -X GET https://127.0.0.1:8443/api/users/ferreal \
  -H 'Content-Type: application/json' \
  | jq

# delete user
$ curl -k -X DELETE https://127.0.0.1:8443/api/users/rustris \
  -H 'Content-Type: application/json' \
  | jq
```

> A unique constraint exists for username, so sending this request twice will return an internal server error (HTTP 500).
