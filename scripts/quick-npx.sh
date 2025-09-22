#!/bin/bash
# Ultra-quick NPX deployment for immediate use
# One-command deployment: curl -fsSL url | bash

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}🚀 CodeFlow Buddy - Ultra Quick NPX Deploy${NC}"
echo "=========================================="

# Check Docker
if ! command -v docker &> /dev/null; then
    echo -e "${RED}❌ Docker required. Install: https://docker.com/get-started${NC}"
    exit 1
fi

# Simple single-container deployment without compose
echo -e "${YELLOW}📦 Deploying with NPX...${NC}"

# Create workspace
mkdir -p ./codeflow-workspace
chmod 755 ./codeflow-workspace

# Kill existing
docker rm -f codeflow-quick 2>/dev/null || true

# Run with NPX - always latest
docker run -d \
    --name codeflow-quick \
    --restart unless-stopped \
    -p 3000:3000 \
    -v "$(pwd)/codeflow-workspace:/workspace" \
    --device /dev/fuse \
    --cap-add SYS_ADMIN \
    --security-opt apparmor:unconfined \
    node:22-alpine sh -c "
        apk add --no-cache fuse fuse-dev python3 make g++ linux-headers curl &&
        echo 'user_allow_other' >> /etc/fuse.conf &&
        addgroup -g 1001 codeflow &&
        adduser -S codeflow -u 1001 -G codeflow &&
        chown -R codeflow:codeflow /workspace &&
        su codeflow -c 'npx @goobits/codeflow-buddy@latest serve --enable-fuse --port 3000 --max-clients 50'
    "

echo -e "${YELLOW}🏥 Waiting for service...${NC}"

# Wait for health
for i in {1..30}; do
    if curl -f http://localhost:3000/health &>/dev/null; then
        echo -e "${GREEN}✅ Service ready!${NC}"
        echo
        echo -e "${GREEN}🌐 Service URL: http://localhost:3000${NC}"
        echo -e "${GREEN}📊 Health Check: http://localhost:3000/health${NC}"
        echo
        echo -e "${YELLOW}💡 Usage:${NC}"
        echo "  View logs: docker logs -f codeflow-quick"
        echo "  Stop:      docker rm -f codeflow-quick"
        echo "  Restart:   docker restart codeflow-quick"
        echo
        echo -e "${GREEN}🎉 Ready for connections!${NC}"
        exit 0
    fi
    echo -n "."
    sleep 2
done

echo -e "${RED}❌ Service failed to start${NC}"
echo "Logs:"
docker logs codeflow-quick
exit 1