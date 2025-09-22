#!/bin/bash
# NPX Docker Deployment Script
# Zero-setup deployment with Docker + NPX

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
CONTAINER_NAME="codeflow-npx"
COMPOSE_FILE="docker-compose.npx.yml"
PORT=${PORT:-3000}
MAX_CLIENTS=${MAX_CLIENTS:-50}

print_header() {
    echo -e "${GREEN}🚀 CodeFlow Buddy NPX Deployment${NC}"
    echo "================================="
}

check_requirements() {
    echo -e "${YELLOW}📋 Checking requirements...${NC}"

    if ! command -v docker &> /dev/null; then
        echo -e "${RED}❌ Docker not found. Please install Docker first.${NC}"
        exit 1
    fi

    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        echo -e "${RED}❌ Docker Compose not found. Please install Docker Compose first.${NC}"
        exit 1
    fi

    echo -e "${GREEN}✅ Docker requirements satisfied${NC}"
}

create_workspaces_dir() {
    echo -e "${YELLOW}📁 Creating workspaces directory...${NC}"
    mkdir -p ./workspaces
    chmod 755 ./workspaces
    echo -e "${GREEN}✅ Workspaces directory ready${NC}"
}

stop_existing() {
    echo -e "${YELLOW}🛑 Stopping existing containers...${NC}"
    docker-compose -f "$COMPOSE_FILE" down 2>/dev/null || true
    docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
    echo -e "${GREEN}✅ Cleanup complete${NC}"
}

build_and_start() {
    echo -e "${YELLOW}🏗️  Building and starting NPX container...${NC}"

    # Set environment variables
    export PORT="$PORT"
    export MAX_CLIENTS="$MAX_CLIENTS"

    # Build and start
    docker-compose -f "$COMPOSE_FILE" up --build -d

    echo -e "${GREEN}✅ Container started${NC}"
}

wait_for_health() {
    echo -e "${YELLOW}🏥 Waiting for service to be healthy...${NC}"

    local max_attempts=30
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if curl -f "http://localhost:$PORT/health" &>/dev/null; then
            echo -e "${GREEN}✅ Service is healthy and ready${NC}"
            return 0
        fi

        echo -n "."
        sleep 2
        ((attempt++))
    done

    echo -e "${RED}❌ Service failed to become healthy${NC}"
    echo "Checking logs..."
    docker-compose -f "$COMPOSE_FILE" logs --tail=20
    exit 1
}

show_status() {
    echo -e "${GREEN}📊 Deployment Status${NC}"
    echo "==================="
    echo -e "Service URL: ${GREEN}http://localhost:$PORT${NC}"
    echo -e "Health Check: ${GREEN}http://localhost:$PORT/health${NC}"
    echo -e "Max Clients: ${GREEN}$MAX_CLIENTS${NC}"
    echo -e "Container: ${GREEN}$CONTAINER_NAME${NC}"
    echo
    echo -e "${YELLOW}📝 Useful Commands:${NC}"
    echo "  View logs:    docker-compose -f $COMPOSE_FILE logs -f"
    echo "  Stop service: docker-compose -f $COMPOSE_FILE down"
    echo "  Restart:      docker-compose -f $COMPOSE_FILE restart"
    echo "  Shell access: docker exec -it $CONTAINER_NAME sh"
}

# Main execution
main() {
    print_header
    check_requirements
    create_workspaces_dir
    stop_existing
    build_and_start
    wait_for_health
    show_status

    echo -e "${GREEN}🎉 NPX deployment complete!${NC}"
}

# Handle command line arguments
case "${1:-}" in
    "stop")
        echo -e "${YELLOW}🛑 Stopping NPX deployment...${NC}"
        docker-compose -f "$COMPOSE_FILE" down
        echo -e "${GREEN}✅ Stopped${NC}"
        ;;
    "logs")
        docker-compose -f "$COMPOSE_FILE" logs -f
        ;;
    "status")
        echo -e "${GREEN}📊 NPX Deployment Status${NC}"
        docker-compose -f "$COMPOSE_FILE" ps
        ;;
    "restart")
        echo -e "${YELLOW}🔄 Restarting NPX deployment...${NC}"
        docker-compose -f "$COMPOSE_FILE" restart
        echo -e "${GREEN}✅ Restarted${NC}"
        ;;
    "")
        main
        ;;
    *)
        echo "Usage: $0 [stop|logs|status|restart]"
        echo "  (no args): Deploy NPX service"
        echo "  stop:      Stop the service"
        echo "  logs:      View service logs"
        echo "  status:    Show service status"
        echo "  restart:   Restart the service"
        exit 1
        ;;
esac