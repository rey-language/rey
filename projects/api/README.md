# Rey Social Media API Application

A high-performance social media backend API written in **Rey** backed by **PostgreSQL**.

---

## Architecture Overview

```
                      +-----------------------------+
                      |   HTTP Clients / cURL / Web |
                      +--------------+--------------+
                                     |
                                     v HTTP JSON
                      +-----------------------------+
                      |    Node.js Server Bridge    | (server.js)
                      +--------------+--------------+
                                     |
                                     v CLI Subprocess IPC
                      +-----------------------------+
                      |       Rey Entry Point       | (main.rey)
                      +--------------+--------------+
                                     |
                                     v
                      +-----------------------------+
                      |      Rey API Router         | (src/router.rey)
                      +--------------+--------------+
                                     |
                                     v
                      +-----------------------------+
                      |    Social Service Domain    | (src/service.rey)
                      +--------------+--------------+
                                     |
                                     v
                      +-----------------------------+
                      |    PostgreSQL Rey Driver    | (src/db.rey)
                      +--------------+--------------+
                                     |
                                     v CTE JSON Queries
                      +-----------------------------+
                      | PostgreSQL 16 (rey_social_db)|
                      +-----------------------------+
```

---

## Directory Structure

```
projects/api/
|-- README.md            # Comprehensive project documentation
|-- schema.sql           # PostgreSQL database schema & indexes
|-- server.js            # Lightweight HTTP bridge server
|-- test_http.sh         # Real HTTP curl integration test suite
|-- main.rey             # Rey CLI & test suite entry point
`-- src/
    |-- db.rey           # PostgreSQL connection & query execution via psql
    |-- json_util.rey    # High-performance zero-dependency JSON field extraction
    |-- router.rey       # Request path & method routing
    `-- service.rey      # Core social business logic & database queries
```

---

## Features & Endpoints

| Method | Endpoint | Description | Status Code |
|---|---|---|---|
| `GET` | `/api/health` | System health check and database statistics | `200 OK` |
| `POST` | `/api/users` | Register a new user (`username`, `name`, `bio`) | `201 Created` / `409 Conflict` |
| `GET` | `/api/users` | List all registered users with follower/post counts | `200 OK` |
| `GET` | `/api/users/:id` | Fetch specific user profile and social stats | `200 OK` / `404 Not Found` |
| `POST` | `/api/posts` | Publish a new post (`user_id`, `content`) | `201 Created` |
| `GET` | `/api/posts` | Global chronological timeline of posts | `200 OK` |
| `GET` | `/api/posts/:id` | Retrieve single post with author metadata | `200 OK` / `404 Not Found` |
| `POST` | `/api/posts/:id/like` | Like a post (`user_id`) | `200 OK` |
| `POST` | `/api/posts/:id/unlike` | Remove like from a post (`user_id`) | `200 OK` |
| `POST` | `/api/follow` | Follow a user (`follower_id`, `following_id`) | `200 OK` |
| `POST` | `/api/unfollow` | Unfollow a user (`follower_id`, `following_id`) | `200 OK` |
| `GET` | `/api/feed/:user_id` | Personalized timeline of posts from followed users | `200 OK` |
| `POST` | `/api/reset` | Reinitialize and wipe database schema | `200 OK` |

---

## Database Schema (`schema.sql`)

PostgreSQL database: `rey_social_db`

- **`users`**: `id`, `username` (unique), `name`, `bio`, `created_at`
- **`posts`**: `id`, `user_id` (foreign key), `content`, `likes_count`, `created_at`
- **`follows`**: `follower_id`, `following_id`, `created_at` (composite PK, self-follow constraint)
- **`likes`**: `user_id`, `post_id`, `created_at` (composite PK)

Includes indexes on `posts(created_at)`, `posts(user_id, created_at)`, `follows(follower_id)`, and `likes(post_id)`.

---

## Running the Application

### 1. Database Setup
Ensure PostgreSQL is running and initialize the database:
```bash
createdb rey_social_db   # if not created
psql -d rey_social_db -f schema.sql
```

### 2. Run Pure Rey Integration Tests
Execute the built-in 10-step integration test suite entirely inside Rey:
```bash
rey-v0 run main.rey -- test
```

### 3. Start the HTTP API Server
```bash
node server.js
# Listening on http://localhost:8080
```

### 4. Run HTTP Integration Tests
Executes real `curl` commands testing all endpoints and error cases against the running server:
```bash
./test_http.sh
```
