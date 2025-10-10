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
# cargo build --release -p cloudhost-tui

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

1. Create cloud folders (virtual links to local directories)
2. Set password in Settings tab
3. Start server from Server tab
4. Access via browser at the provided URL


