# CloudHost Server

The server component of CloudHost - provides web interface and REST API for file hosting.

## API Endpoints

### Authentication
- `POST /api/login` - Login with password, returns JWT token
  ```json
  {"password": "your_password"}
  ```

### Server Status
- `GET /api` - Get server status and cloud folder list
  ```json
  {
    "status": "running",
    "cloud": {
      "name": "MyCloud",
      "cloud_folders": [
        {"name": "cloudfolder1"},
        {"name": "cloudfolder2"}
      ]
    },
    "timestamp": "2024-01-01T00:00:00Z"
  }
  ```

### Cloud Folder Management
- `GET /api/{cloud_folder_name}` - Get specific cloud folder information
- `GET /api/{cloud_folder_name}/files` - List files in cloud folder (JSON)
- `GET /api/{cloud_folder_name}/files/*path` - Browse files/directories (JSON)
- `GET /api/{cloud_folder_name}/static/*path` - Download static files

## Authentication

All API endpoints require authentication via:
- **Authorization Header**: `Bearer <jwt_token>`
- **Cookie**: `auth_token_{port}=<jwt_token>` (port-specific cookies for multi-cloud support)

## Web Interface Routes

- `/` - Main dashboard showing all cloud folders (requires login)
- `/login` - Login page
- `/web/{cloud_folder_name}/files` - Browse cloud folder contents (HTML)
- `/web/{cloud_folder_name}/files/*path` - View/download files (HTML)

## Error Responses

API endpoints return JSON error responses:
```json
{
  "error": "Error Type",
  "message": "Human readable description"
}
```
