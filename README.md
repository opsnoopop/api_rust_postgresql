# Rust API with PostgreSQL

A simple Rust API application and PostgreSQL, containerized with Docker.


## Technology Stack

**Rust Container: FROM debian:bookworm-slim**
- OS: Debian GNU/Linux 12 (bookworm) # Builder rust:1.85-slim-bookworm, Runtime debian:bookworm-slim
- Rust: 1.85
- cargo new myapp
- cd myapp
- cargo add axum@0.7 --features full
- cargo add tokio@1.38 --features full
- cargo add serde@1 --features derive
- cargo add serde_json@1
- cargo add sqlx@0.7 --features runtime-tokio-rustls,postgresql
- cargo add dotenvy@0.15

**PostgreSQL Container: FROM postgres:17.5**
- OS Debian GNU/Linux 12 (bookworm): 12
- PostgreSQL: 17.5

**Adminer Container: FROM adminer:5-standalone**
- OS Alpine Linux: 3.22.1
- Adminer: 5.3.0

**Grafana/k6 Container: FROM grafana/k6:1.1.0**
- OS Alpine Linux: 3.22.0
- Grafana/k6: 1.1.0


## Getting Started

### 1. Clone the Repository
```bash
git clone https://github.com/opsnoopop/api_rust_postgresql.git
```

### 2. Navigate to Project Directory
```bash
cd api_rust_postgresql
```

### 3. Start the Application
```bash
docker compose up -d --build
```

### 4. Create table users
```bash
docker exec -i container_postgresql sh -c "PGPASSWORD='testpass' psql -U testuser -d testdb -c '
CREATE TABLE IF NOT EXISTS public.users (
  user_id SERIAL PRIMARY KEY,
  username VARCHAR(50) NOT NULL,
  email VARCHAR(100) NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);'"
```


## API Endpoints

### Health Check
```bash
curl -X GET http://localhost:3000/
```

### Create user
```bash
curl -X POST http://localhost:3000/users -H 'Content-Type: application/json' -d '{"username":"optest","email":"opsnoopop@hotmail.com"}'
```

### Get user
```bash
curl -X GET http://localhost:3000/users/1
```


## Test Performance by sysbench

### sysbench e.g.
```text
step 1 prepare
step 2 run test someting
step 3 cleanup

sysbench \
...
oltp_read_write prepare; # step 1 prepare สร้าง table

sysbench \
...
oltp_read_write run;     # step 2 run test อ่าน เขียน พร้อมกัน

sysbench \
...
oltp_read_only run;      # step 2 run test อ่าน อย่างเดียว

sysbench \
...
oltp_write_only run;     # step 2 run test เขียน อย่างเดียว

sysbench \
...
oltp_update_index run;   # step 2 run test update index

sysbench \
...
oltp_point_select run;   # step 2 run test query แบบเลือก row เดียว

sysbench \
...
oltp_delete run;         # step 2 run test delete rows

sysbench \
...
oltp_read_write cleanup; # step 3 cleanup ลบ table
```

### sysbench step 1 prepare
```bash
docker run \
--name container_ubuntu_tool \
--rm \
-it \
--network global_rust \
opsnoopop/ubuntu-tool:1.0 \
sysbench \
--threads=2 \
--time=10 \
--db-driver="pgsql" \
--pgsql-host="container_postgresql" \
--pgsql-port=5432 \
--pgsql-user="testuser" \
--pgsql-password="testpass" \
--pgsql-db="testdb" \
--tables=10 \
--table-size=100000 \
oltp_read_write prepare;
```

### sysbench step 2 run test
```bash
docker run \
--name container_ubuntu_tool \
--rm \
-it \
--network global_rust \
opsnoopop/ubuntu-tool:1.0 \
sysbench \
--threads=2 \
--time=10 \
--db-driver="pgsql" \
--pgsql-host="container_postgresql" \
--pgsql-port=5432 \
--pgsql-user="testuser" \
--pgsql-password="testpass" \
--pgsql-db="testdb" \
--tables=10 \
--table-size=100000 \
oltp_read_write run > sysbench_raw_$(date +"%Y%m%d_%H%M%S").txt
```

### sysbench step 3 cleanup
```bash
docker run \
--name container_ubuntu_tool \
--rm \
-it \
--network global_rust \
opsnoopop/ubuntu-tool:1.0 \
sysbench \
--threads=2 \
--time=10 \
--db-driver="pgsql" \
--pgsql-host="container_postgresql" \
--pgsql-port=5432 \
--pgsql-user="testuser" \
--pgsql-password="testpass" \
--pgsql-db="testdb" \
--tables=10 \
--table-size=100000 \
oltp_read_write cleanup;
```


## Test Performance by grafana/k6

### grafana/k6 test Health Check
```bash
docker run \
--name container_k6 \
--rm \
-it \
--network global_rust \
-v ./k6/:/k6/ \
grafana/k6:1.1.0 \
run /k6/k6_1_ramping_health_check.js
```

### grafana/k6 test Insert Create user
```bash
docker run \
--name container_k6 \
--rm \
-it \
--network global_rust \
-v ./k6/:/k6/ \
grafana/k6:1.1.0 \
run /k6/k6_2_ramping_create_user.js
```

### grafana/k6 test Select Get user by id
```bash
docker run \
--name container_k6 \
--rm \
-it \
--network global_rust \
-v ./k6/:/k6/ \
grafana/k6:1.1.0 \
run /k6/k6_3_ramping_get_user_by_id.js
```

### check entrypoint grafana/k6
```bash
docker run \
--name container_k6 \
--rm \
-it \
--entrypoint \
/bin/sh grafana/k6:1.1.0
```


## Stop the Application

### Truncate table users
```bash
docker exec -i container_postgresql sh -c "PGPASSWORD='testpass' psql -U testuser -d testdb -c '
Truncate public.users RESTART IDENTITY;'"
```

### Delete table users
```bash
docker exec -i container_postgresql sh -c "PGPASSWORD='testpass' psql -U testuser -d testdb -c '
DELETE FROM public.users;'"
```

### Stop the Application
```bash
docker compose down
```