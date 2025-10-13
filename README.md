# CloudHost

A file server that helps users host their files locally. 

Currently features a terminal user interface (TUI) with more interfaces and features planned for the future.

## Installation

### Option 1: Pre-built binary (Recommended)
Download the latest release from [GitHub Releases](https://github.com/StepanZagray/CloudHost/releases)

### Option 2: Build from source
```bash
git clone https://github.com/StepanZagray/CloudHost.git
cd CloudHost

# Build TUI (terminal interface) - EASY WAY
cargo tui
./target/release/cloudhost-tui

# Alternative: Build specific package
# cargo build --release -p cloudhost

# Future UIs (when available)
# cargo gui    # Desktop GUI
# cargo web    # Web UI
```

### Option 3: Install via cargo
```bash
cargo install --git https://github.com/StepanZagray/CloudHost.git --bin cloudhost-tui
cloudhost-tui
```

## Setup

### Local
1. Create cloud folders (virtual links to local directories)
2. Set password in Settings tab
3. Start server from Server tab
4. Access via browser at the provided URL by server logs

### Internet
1. complete first 3 steps from local
2. Set up a cloudflare tunnel by doing steps from [docs](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/get-started/create-remote-tunnel/)
3. If you do not want to use/buy domain, you can use cloudflare generic tunnel domain by opening terminal in admin mode and writing `cloudflared tunnel --url http://localhost:3000`
4. Access via provided link by output of step 3



## To-Do features:
- upload functionality
- users with different permissions(look, download, upload)
- make it possible to share cloudfolders on several devices, and sync files between them

