# Auth0 User Management Frontend

A web application for managing Auth0 users, connections, applications, and logs.

Built with:
- **Rust** + **Axum** for the backend
- **Askama** for HTML templating
- **Bootstrap 5** for styling
- **htmx** for interactivity without JavaScript frameworks
- **auth0-mgmt-api** for Auth0 Management API integration

## Features

- **Users**: List, create, view, edit, block/unblock, and delete users
- **Connections**: View identity provider connections
- **Applications**: View OAuth applications
- **Logs**: View authentication logs with search

## Setup

1. Create a Machine-to-Machine application in Auth0:
   - Go to Applications → Create Application → Machine to Machine
   - Select the Auth0 Management API
   - Grant the following scopes:
     - `read:users`, `create:users`, `update:users`, `delete:users`
     - `read:connections`
     - `read:clients`
     - `read:logs`

2. Copy `.env.example` to `.env` and fill in your credentials:
   ```bash
   cp .env.example .env
   ```

3. Run the application:
   ```bash
   cargo run
   ```

4. Open http://localhost:3000 in your browser

## Development

```bash
# Run with hot reload (requires cargo-watch)
cargo watch -x run

# Run with debug logging
RUST_LOG=debug cargo run
```

## Project Structure

```
.
├── src/
│   └── main.rs          # Axum routes and handlers
├── templates/
│   ├── base.html        # Base layout with navigation
│   ├── index.html       # Dashboard
│   ├── users/           # User management templates
│   ├── connections/     # Connection templates
│   ├── applications/    # Application templates
│   └── logs/            # Log viewer templates
├── Cargo.toml
└── .env
```
