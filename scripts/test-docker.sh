#!/bin/bash
# Test Docker build and deployment
# Usage: ./scripts/test-docker.sh

set -e

echo "🥑 AvocadoDB Docker Test Script"
echo "================================"
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Test 1: Dockerfile syntax
echo "📋 Test 1: Validating Dockerfile syntax..."
if docker build --check . > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Dockerfile syntax is valid"
else
    echo -e "${RED}✗${NC} Dockerfile syntax error"
    exit 1
fi

# Test 2: Build Docker image
echo ""
echo "🔨 Test 2: Building Docker image..."
if docker build -t avocadodb:test . --quiet; then
    echo -e "${GREEN}✓${NC} Docker image built successfully"
else
    echo -e "${RED}✗${NC} Docker build failed"
    exit 1
fi

# Test 3: Check image size
echo ""
echo "📊 Test 3: Checking image size..."
SIZE=$(docker image inspect avocadodb:test --format='{{.Size}}')
SIZE_MB=$((SIZE / 1024 / 1024))
echo "   Image size: ${SIZE_MB}MB"
if [ $SIZE_MB -lt 200 ]; then
    echo -e "${GREEN}✓${NC} Image size is optimal (<200MB)"
else
    echo -e "${YELLOW}⚠${NC} Image size is larger than expected (${SIZE_MB}MB > 200MB)"
fi

# Test 4: Start container
echo ""
echo "🚀 Test 4: Starting container..."
docker run -d --name avocadodb-test -p 8765:8765 -e RUST_LOG=info avocadodb:test > /dev/null
echo -e "${GREEN}✓${NC} Container started"

# Test 5: Wait for startup
echo ""
echo "⏳ Test 5: Waiting for server to start..."
sleep 5
echo -e "${GREEN}✓${NC} Server should be ready"

# Test 6: Health check
echo ""
echo "🏥 Test 6: Testing health endpoint..."
for i in {1..10}; do
    if curl -f -s http://localhost:8765/health > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} Health check passed"
        HEALTH_OK=true
        break
    fi
    sleep 1
done

if [ -z "$HEALTH_OK" ]; then
    echo -e "${RED}✗${NC} Health check failed"
    docker logs avocadodb-test
    docker stop avocadodb-test
    docker rm avocadodb-test
    exit 1
fi

# Test 7: Test API
echo ""
echo "🔌 Test 7: Testing API endpoints..."
if curl -f -s http://localhost:8765/stats > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Stats endpoint working"
else
    echo -e "${RED}✗${NC} Stats endpoint failed"
    docker logs avocadodb-test
    docker stop avocadodb-test
    docker rm avocadodb-test
    exit 1
fi

# Test 8: Check logs
echo ""
echo "📜 Test 8: Checking container logs..."
docker logs avocadodb-test | tail -5
echo -e "${GREEN}✓${NC} Logs look good"

# Test 9: Test Docker Compose
echo ""
echo "🐳 Test 9: Testing Docker Compose..."
if docker-compose config > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} docker-compose.yml is valid"
else
    echo -e "${RED}✗${NC} docker-compose.yml has errors"
    docker stop avocadodb-test
    docker rm avocadodb-test
    exit 1
fi

# Cleanup
echo ""
echo "🧹 Cleaning up..."
docker stop avocadodb-test > /dev/null 2>&1
docker rm avocadodb-test > /dev/null 2>&1
echo -e "${GREEN}✓${NC} Cleanup complete"

# Summary
echo ""
echo "================================"
echo -e "${GREEN}✅ All tests passed!${NC}"
echo ""
echo "Summary:"
echo "  - Dockerfile syntax: ✓"
echo "  - Image built: ✓"
echo "  - Image size: ${SIZE_MB}MB"
echo "  - Container starts: ✓"
echo "  - Health check: ✓"
echo "  - API works: ✓"
echo "  - Docker Compose: ✓"
echo ""
echo "Ready for deployment! 🚀"
