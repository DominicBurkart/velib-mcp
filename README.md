# Velib MCP Server

A high-performance Model Context Protocol (MCP) server providing access to Paris Velib bike sharing data for AI assistants.

## 🚀 Quick Start with Claude Code

Install and configure the Velib MCP server for Claude Code in one command:

```bash
curl -fsSL https://raw.githubusercontent.com/dominicburkart/velib-mcp/main/install.sh | bash
```

**That's it!** 🎉 The server is now ready to use with Claude Code.

### What this gives you:
- **Real-time bike availability** at all Paris Velib stations
- **Smart station search** by location, name, or area
- **Journey planning** with optimal pickup/dropoff suggestions
- **Area statistics** for bike sharing analysis
- **Seamless Claude Code integration** with automatic configuration

## Overview

This project exposes two key Parisian datasets through MCP:
- **Real-time availability**: Current bike and dock availability at stations
- **Station locations**: Geographic information and details about all Velib stations

## 🔧 Other AI Platforms

<details>
<summary><strong>ChatGPT • Cursor • Le Chat • Windsurf</strong></summary>

### Manual Installation
```bash
git clone https://github.com/dominicburkart/velib-mcp.git
cd velib-mcp
cargo build --release
```

### Platform-Specific Setup
- **Cursor**: Add to `~/.cursor/mcp_servers.json`
- **Windsurf**: Add to `~/.windsurf/mcp_servers.json`  
- **ChatGPT/Le Chat**: Use HTTP endpoint `http://localhost:8080`

See [mcp-config-examples.json](mcp-config-examples.json) for detailed configuration examples.
</details>

## Data Sources

- [Velib Real-time Availability](https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/)
- [Velib Station Locations](https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/)

## Development

### Prerequisites

- Rust (latest stable)
- OpenSSL development libraries
- pkg-config

### Setup

```bash
git clone https://github.com/dominicburkart/velib-mcp.git
cd velib-mcp
cargo build
```

### Testing

```bash
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo audit
```

### Podman

```bash
# Build container image
podman build -t velib-mcp .

# Run container
podman run -p 8080:8080 velib-mcp
```

## Deployment

The project is configured for deployment to Scaleway Container Serverless via GitHub Actions on pushes to the main branch.

## Architecture

- **Language**: Rust
- **Deployment**: Scaleway Container Serverless  
- **CI/CD**: GitHub Actions
- **Development**: Test-Driven Development approach
- **Container**: Distroless Debian base image

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT License

at your option.