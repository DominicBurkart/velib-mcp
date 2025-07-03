# Velib MCP Server

[![Test Coverage](https://img.shields.io/badge/coverage-check%20actions-brightgreen)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Tests](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![Security Audit](https://img.shields.io/badge/security-audit%20passing-brightgreen)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](https://github.com/DominicBurkart/velib-mcp#license)
[![Rust Version](https://img.shields.io/badge/rust-latest%20stable-orange)](https://www.rust-lang.org/)
[![Deploy Status](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml/badge.svg)](https://github.com/DominicBurkart/velib-mcp/actions/workflows/deploy.yml)

A high-performance Model Context Protocol (MCP) server providing access to Paris Velib bike sharing data for AI assistants.

## Overview

This project exposes two key Parisian datasets through MCP:
- **Real-time availability**: Current bike and dock availability at stations
- **Station locations**: Geographic information and details about all Velib stations

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