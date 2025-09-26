# CloudTUI Counter App

A terminal-based counter application with SQLite database integration and client/server architecture.

## Features

- **Mode Selection**: Choose between Server or Client mode
- **Server Mode**: Runs a local web server with SQLite database storage
- **Client Mode**: Connects to a remote server and updates counter via HTTP API
- **Database Integration**: Uses SQLite to persist counter values
- **Web API**: RESTful endpoints for counter operations

## Usage

### Running the Application

```bash
cargo run
```

### Mode Selection

When you start the application, you'll see a mode selection screen:

1. **Server Mode** (Press '1'):
   - Initializes SQLite database (`counter.db`)
   - Starts web server on `http://localhost:3000`
   - Counter is stored in database
   - Use arrow keys to increment/decrement counter

2. **Client Mode** (Press '2'):
   - Connects to server at `http://localhost:3000`
   - Updates counter via HTTP API
   - Use arrow keys to increment/decrement counter
   - Press 'R' to refresh counter from server

### Controls

- **Mode Selection**: '1' for Server, '2' for Client, 'Q' to quit
- **Server/Client**: Left/Right arrows to change counter, 'Q' to quit
- **Client Only**: 'R' to refresh counter from server

## API Endpoints

When running in server mode, the following endpoints are available:

- `GET /` - API documentation
- `GET /counter` - Get current counter value
- `POST /counter` - Update counter (send `{"delta": <number>}`)

## Database Schema

The application creates a SQLite database with the following schema:

```sql
CREATE TABLE counter (
    id INTEGER PRIMARY KEY,
    value INTEGER NOT NULL DEFAULT 0
);
```

## Architecture

- **TUI**: Built with Ratatui for terminal interface
- **Web Server**: Axum with Tower middleware
- **Database**: SQLite with SQLx
- **HTTP Client**: Reqwest for client-server communication
- **Async Runtime**: Tokio for async operations

## Dependencies

- `ratatui` - Terminal UI framework
- `axum` - Web framework
- `sqlx` - SQL toolkit with async support
- `reqwest` - HTTP client
- `tokio` - Async runtime
- `tower` - Middleware framework
