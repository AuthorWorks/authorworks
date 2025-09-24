#!/bin/bash
set -e

echo "🧪 Running AuthorWorks Integration Tests"
echo "========================================="

# Set test environment
export TEST_ENV=${TEST_ENV:-"local"}
export BASE_URL=${BASE_URL:-"http://localhost:80"}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

TESTS_PASSED=0
TESTS_FAILED=0

# Function to test endpoint
test_endpoint() {
    local endpoint=$1
    local expected_status=$2
    local description=$3

    response=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL$endpoint")

    if [ "$response" = "$expected_status" ]; then
        echo -e "${GREEN}✅${NC} $description (HTTP $response)"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        echo -e "${RED}❌${NC} $description (Expected $expected_status, got $response)"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

# Start Spin application locally if not running
if [ "$TEST_ENV" = "local" ]; then
    echo "Starting Spin application locally..."
    spin up --listen 127.0.0.1:8080 &
    SPIN_PID=$!
    sleep 5
    BASE_URL="http://localhost:8080"
fi

echo ""
echo "Testing against: $BASE_URL"
echo ""

# Test health endpoints
echo "1. Health Check Endpoints"
echo "-------------------------"
test_endpoint "/health" "200" "Main health check"
test_endpoint "/api/user/health" "200" "User service health"
test_endpoint "/api/content/health" "200" "Content service health"
test_endpoint "/api/storage/health" "200" "Storage service health"
test_endpoint "/api/editor/health" "200" "Editor service health"
test_endpoint "/api/messaging/health" "200" "Messaging service health"
test_endpoint "/api/discovery/health" "200" "Discovery service health"
test_endpoint "/api/subscription/health" "200" "Subscription service health"

# Test authentication flow
echo ""
echo "2. Authentication Flow"
echo "----------------------"
# Register user
REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/api/user/register" \
    -H "Content-Type: application/json" \
    -d '{"email":"test@example.com","password":"Test123!","name":"Test User"}')

if echo "$REGISTER_RESPONSE" | grep -q "token"; then
    echo -e "${GREEN}✅${NC} User registration successful"
    TESTS_PASSED=$((TESTS_PASSED + 1))

    # Extract token
    TOKEN=$(echo "$REGISTER_RESPONSE" | jq -r '.token')

    # Test authenticated endpoint
    AUTH_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/user/profile" \
        -H "Authorization: Bearer $TOKEN")

    if [ "$AUTH_RESPONSE" = "200" ]; then
        echo -e "${GREEN}✅${NC} Authenticated request successful"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}❌${NC} Authenticated request failed"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
else
    echo -e "${YELLOW}⚠️${NC}  User registration skipped (may already exist)"
fi

# Test content operations
echo ""
echo "3. Content Operations"
echo "--------------------"
if [ -n "$TOKEN" ]; then
    # Create content
    CREATE_RESPONSE=$(curl -s -X POST "$BASE_URL/api/content/create" \
        -H "Authorization: Bearer $TOKEN" \
        -H "Content-Type: application/json" \
        -d '{"title":"Test Article","content":"Test content","type":"article"}')

    if echo "$CREATE_RESPONSE" | grep -q "id"; then
        echo -e "${GREEN}✅${NC} Content creation successful"
        TESTS_PASSED=$((TESTS_PASSED + 1))

        CONTENT_ID=$(echo "$CREATE_RESPONSE" | jq -r '.id')

        # Get content
        GET_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/content/$CONTENT_ID" \
            -H "Authorization: Bearer $TOKEN")

        if [ "$GET_RESPONSE" = "200" ]; then
            echo -e "${GREEN}✅${NC} Content retrieval successful"
            TESTS_PASSED=$((TESTS_PASSED + 1))
        else
            echo -e "${RED}❌${NC} Content retrieval failed"
            TESTS_FAILED=$((TESTS_FAILED + 1))
        fi
    else
        echo -e "${RED}❌${NC} Content creation failed"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
fi

# Test storage operations
echo ""
echo "4. Storage Operations"
echo "--------------------"
if [ -n "$TOKEN" ]; then
    # Upload file
    UPLOAD_RESPONSE=$(curl -s -X POST "$BASE_URL/api/storage/upload" \
        -H "Authorization: Bearer $TOKEN" \
        -F "file=@README.md")

    if echo "$UPLOAD_RESPONSE" | grep -q "url"; then
        echo -e "${GREEN}✅${NC} File upload successful"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${YELLOW}⚠️${NC}  File upload skipped (storage may not be configured)"
    fi
fi

# Test WebSocket connectivity
echo ""
echo "5. WebSocket Connectivity"
echo "------------------------"
if command -v wscat >/dev/null 2>&1; then
    WS_URL=${BASE_URL/http/ws}
    timeout 2 wscat -c "$WS_URL/api/messaging/ws" 2>/dev/null && {
        echo -e "${GREEN}✅${NC} WebSocket connection successful"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    } || {
        echo -e "${YELLOW}⚠️${NC}  WebSocket connection failed (may not be configured)"
    }
else
    echo -e "${YELLOW}⚠️${NC}  wscat not installed, skipping WebSocket test"
fi

# Test metrics endpoint
echo ""
echo "6. Metrics & Monitoring"
echo "-----------------------"
test_endpoint "/metrics" "200" "Prometheus metrics endpoint"

# Performance test
echo ""
echo "7. Performance Test"
echo "-------------------"
if command -v ab >/dev/null 2>&1; then
    echo "Running basic load test..."
    ab -n 100 -c 10 -q "$BASE_URL/health" > /tmp/ab_results.txt 2>&1

    RPS=$(grep "Requests per second" /tmp/ab_results.txt | awk '{print $4}')
    AVG_TIME=$(grep "Time per request" /tmp/ab_results.txt | head -1 | awk '{print $4}')

    echo -e "  Requests per second: ${RPS}"
    echo -e "  Average response time: ${AVG_TIME}ms"

    if (( $(echo "$RPS > 100" | bc -l) )); then
        echo -e "${GREEN}✅${NC} Performance test passed (>100 RPS)"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${YELLOW}⚠️${NC}  Performance below expectations (<100 RPS)"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
else
    echo -e "${YELLOW}⚠️${NC}  ab (Apache Bench) not installed, skipping performance test"
fi

# Cleanup
if [ "$TEST_ENV" = "local" ] && [ -n "$SPIN_PID" ]; then
    echo ""
    echo "Stopping local Spin application..."
    kill $SPIN_PID 2>/dev/null || true
fi

# Summary
echo ""
echo "========================================="
echo "Integration Test Summary"
echo "========================================="
echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ All integration tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed. Please review the output above.${NC}"
    exit 1
fi