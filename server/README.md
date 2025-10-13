# CloudHost Server

The server component of CloudHost - provides web interface and REST API for file hosting.

## API Endpoints

### Authentication
- `POST /api/login` - Login with password, returns JWT token
  ```json
  {"password": "your_password"}
  ```

### Server Status
- `GET /api/status` - Get server status and cloudfolder list
- `GET /api/cloudfolders` - Same as `/api/status`

### Cloudfolder Management
- `GET /api/cloudfolders/{name}` - Get cloudfolder information
- `GET /api/cloudfolders/{name}/files` - List files in cloudfolder
- `GET /api/cloudfolders/{name}/files/*path` - Browse files/directories

## Authentication

All API endpoints require authentication via:
- **Authorization Header**: `Bearer <jwt_token>`
- **Cookie**: `auth_token=<jwt_token>`

## Web Interface Routes

- `/` - Main dashboard (requires login)
- `/login` - Login page
- `/{cloudfolder_name}` - Browse cloudfolder contents
- `/{cloudfolder_name}/files/*path` - View/download files

## Error Responses

API endpoints return JSON error responses:
```json
{
  "error": "Error Type",
  "message": "Human readable description"
}
```
