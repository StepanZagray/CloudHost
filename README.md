# CloudHost TUI

A secure file server with a terminal user interface (TUI) that helps users host their files locally with a modern web interface and REST API.

**This is the TUI version of CloudHost** - featuring a terminal user interface with vim-like navigation for management and a web interface for file browsing with JWT-based authentication.

## Installation

### Option 1: Pre-built binary (Recommended)
Download the latest release from [GitHub Releases](https://github.com/StepanZagray/cloudhost-tui/releases)

#### Windows Defender Warning
If Windows Defender shows a virus warning:
1. Click "More info" 
2. Click "Run anyway"
3. Or add an exception in Windows Security

This is a false positive. The source code is available for inspection.

### Option 2: Build from source
```bash
git clone https://github.com/StepanZagray/cloudhost-tui.git
cd cloudhost-tui

# Build the TUI application
cargo build --release
./target/release/cloudhost-tui
```

### Option 3: Install via cargo
```bash
cargo install --git https://github.com/StepanZagray/cloudhost-tui.git --bin cloudhost-tui
cloudhost-tui
```

## Setup

### Local
1. **Add Folders**: In **Storage**, add the local folders you want CloudHost to serve
2. **Create Cloud**: Select those folders, then create a cloud (a group of folders served together)
3. **Set Password**: Set a secure password for your cloud
4. **Start Server**: Go to **Dashboard** and start the cloud service
5. **Access Files**: Use the provided URL to access your files via web browser

### Core TUI keys

| Key | Storage action |
| --- | --- |
| `a` | Add a folder |
| `n` | Create a cloud from the selected folders |
| `e` | Edit the focused folder or cloud |
| `d` / `x` | Remove the focused folder or cloud |
| `v` | Include or exclude the focused folder |
| `A` | Select all folders |
| `Tab` / `Shift+Tab` | Move focus between panels |

### Web Interface
- **Main Dashboard**: `http://localhost:PORT/` - Lists all cloud folders
- **File Browser**: `http://localhost:PORT/web/{cloud_folder_name}/files` - Browse files in a specific cloud folder
- **Login**: `http://localhost:PORT/login` - Secure login with your cloud password

### Internet Access
1. Complete the local setup steps above
2. Download `cloudflared` from [Cloudflare](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/)
3. Set up a Cloudflare tunnel:
   - Follow the [official documentation](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/get-started/create-remote-tunnel/)
   - Or use the generic tunnel domain: `cloudflared tunnel --url http://localhost:PORT`
4. Access your files via the provided Cloudflare tunnel URL



## To-Do features:
- passwords with different permissions(download only, all)
- make it possible to share cloudfolders on several devices, and sync files between them
