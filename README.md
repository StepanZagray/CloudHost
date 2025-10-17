# CloudHost

A secure file server that helps users host their files locally with a modern web interface and REST API.

Features a terminal user interface (TUI) with vim-like navigation for management and a web interface for file browsing with JWT-based authentication.

## Installation

### Option 1: Pre-built binary (Recommended)
Download the latest release from [GitHub Releases](https://github.com/StepanZagray/CloudHost/releases)

#### Windows Defender Warning
If Windows Defender shows a virus warning:
1. Click "More info" 
2. Click "Run anyway"
3. Or add an exception in Windows Security

This is a false positive. The source code is available for inspection.

### Option 2: Build from source
```bash
git clone https://github.com/StepanZagray/CloudHost.git
cd CloudHost

# Build TUI (terminal interface) - Currently the only available UI
cargo build --release -p cloudhost-tui
./target/release/cloudhost-tui

# Future UIs (when available)
# cargo build --release -p cloudhost-gui    # Desktop GUI
# cargo build --release -p cloudhost-web    # Web UI
```

### Option 3: Install via cargo
```bash
cargo install --git https://github.com/StepanZagray/CloudHost.git --bin cloudhost-tui
cloudhost-tui
```

## Setup

### Local
1. **Create Cloud Folders**: In the Folders tab, create folders that link to your local directories
2. **Create Cloud**: Select the folders you want to include and create a cloud (group of folders that will be served together)
3. **Set Password**: Set a secure password for your cloud
4. **Start Server**: Go to the Clouds tab and start your cloud server
5. **Access Files**: Use the provided URL to access your files via web browser

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
- upload functionality
- users with different permissions(look, download, upload)
- make it possible to share cloudfolders on several devices, and sync files between them

