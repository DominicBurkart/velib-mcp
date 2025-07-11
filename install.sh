#!/bin/bash

# Velib MCP Server - One-Line Installation Script
# Installs and configures the Velib MCP server for Claude Code

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO_URL="https://github.com/dominicburkart/velib-mcp.git"
INSTALL_DIR="$HOME/velib-mcp"
CONFIG_DIR="$HOME/.config/claude-code"
MCP_CONFIG_FILE="$CONFIG_DIR/mcp_servers.json"
SERVER_PORT=8080

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if Rust is installed
check_rust() {
    if ! command -v rustc &> /dev/null; then
        print_status "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        print_success "Rust installed successfully"
        print_warning "Note: You may need to restart your terminal or run 'source ~/.cargo/env' to use Rust in new sessions"
    else
        print_success "Rust already installed"
    fi
}

# Check for required system dependencies
check_dependencies() {
    print_status "Checking system dependencies..."
    
    # Check for OpenSSL dev libraries
    if ! pkg-config --exists openssl; then
        print_warning "OpenSSL development libraries not found"
        print_status "Please install OpenSSL development libraries:"
        print_status "  Ubuntu/Debian: sudo apt-get install libssl-dev pkg-config"
        print_status "  CentOS/RHEL: sudo yum install openssl-devel pkg-config"
        print_status "  macOS: brew install openssl pkg-config"
        exit 1
    fi
    
    print_success "System dependencies verified"
}

# Clone or update the repository
setup_repository() {
    print_status "Setting up Velib MCP repository..."
    
    if [ -d "$INSTALL_DIR" ]; then
        print_status "Repository already exists, updating..."
        cd "$INSTALL_DIR"
        git pull origin main
    else
        print_status "Cloning repository..."
        git clone "$REPO_URL" "$INSTALL_DIR"
        cd "$INSTALL_DIR"
    fi
    
    print_success "Repository setup complete"
}

# Build the MCP server
build_server() {
    print_status "Building Velib MCP server..."
    cd "$INSTALL_DIR"
    
    cargo build --release
    
    print_success "Server built successfully"
}

# Create Claude Code MCP configuration
create_claude_config() {
    print_status "Creating Claude Code MCP configuration..."
    
    # Create config directory if it doesn't exist
    mkdir -p "$CONFIG_DIR"
    
    # Create or update MCP servers configuration
    cat > "$MCP_CONFIG_FILE" << EOF
{
  "mcpServers": {
    "velib": {
      "command": "$INSTALL_DIR/target/release/velib-mcp",
      "env": {
        "IP": "127.0.0.1",
        "PORT": "$SERVER_PORT"
      }
    }
  }
}
EOF
    
    print_success "Claude Code configuration created at $MCP_CONFIG_FILE"
}

# Start the server for testing
test_server() {
    print_status "Testing server startup..."
    
    cd "$INSTALL_DIR"
    
    # Start server in background with timeout
    timeout 10s sh -c "IP=127.0.0.1 PORT=$SERVER_PORT ./target/release/velib-mcp" &
    SERVER_PID=$!
    
    # Give server time to start
    sleep 3
    
    # Check if server is running
    if kill -0 $SERVER_PID 2>/dev/null; then
        print_success "Server started successfully"
        kill $SERVER_PID
        wait $SERVER_PID 2>/dev/null
    else
        print_error "Server failed to start"
        exit 1
    fi
}

# Print final instructions
print_instructions() {
    echo ""
    echo "🎉 Velib MCP Server installation complete!"
    echo ""
    echo "📋 Next steps:"
    echo "1. Start the server:"
    echo "   cd $INSTALL_DIR && IP=127.0.0.1 PORT=$SERVER_PORT ./target/release/velib-mcp"
    echo ""
    echo "2. The server will run on http://localhost:$SERVER_PORT"
    echo ""
    echo "3. Claude Code is configured to use the server automatically"
    echo "   Configuration file: $MCP_CONFIG_FILE"
    echo ""
    echo "4. Available MCP tools:"
    echo "   - find_nearby_stations: Find Velib stations near a location"
    echo "   - get_station_by_code: Get details for a specific station"
    echo "   - search_stations_by_name: Search stations by name"
    echo "   - get_area_statistics: Get statistics for an area"
    echo "   - plan_bike_journey: Plan a bike journey with stations"
    echo ""
    echo "5. Available MCP resources:"
    echo "   - velib://stations/reference: Station locations and metadata"
    echo "   - velib://stations/realtime: Real-time availability data"
    echo "   - velib://stations/complete: Combined reference and real-time data"
    echo ""
    echo "📚 Documentation: https://github.com/dominicburkart/velib-mcp"
    echo "🐛 Issues: https://github.com/dominicburkart/velib-mcp/issues"
}

# Main installation flow
main() {
    echo "🚀 Installing Velib MCP Server for Claude Code..."
    echo ""
    
    check_rust
    check_dependencies
    setup_repository
    build_server
    create_claude_config
    test_server
    
    print_instructions
}

# Run main function
main "$@"