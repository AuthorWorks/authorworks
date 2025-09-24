#!/bin/bash
set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "🔍 AuthorWorks Pre-Deployment Verification"
echo "=========================================="

ERRORS=0
WARNINGS=0

check_command() {
    if command -v $1 >/dev/null 2>&1; then
        echo -e "${GREEN}✅${NC} $1 is installed"
        return 0
    else
        echo -e "${RED}❌${NC} $1 is not installed"
        ERRORS=$((ERRORS + 1))
        return 1
    fi
}

check_file() {
    if [ -f "$1" ]; then
        echo -e "${GREEN}✅${NC} $1 exists"
        return 0
    else
        echo -e "${RED}❌${NC} $1 is missing"
        ERRORS=$((ERRORS + 1))
        return 1
    fi
}

check_env() {
    if [ -z "${!1}" ]; then
        echo -e "${YELLOW}⚠️${NC}  $1 is not set (using default)"
        WARNINGS=$((WARNINGS + 1))
        return 1
    else
        echo -e "${GREEN}✅${NC} $1 is set"
        return 0
    fi
}

echo ""
echo "1. Checking required tools..."
echo "------------------------------"
check_command kubectl
check_command helm
check_command docker
check_command spin
check_command rustc
check_command cargo

echo ""
echo "2. Checking Rust targets..."
echo "------------------------------"
if rustup target list --installed | grep -q wasm32-wasi; then
    echo -e "${GREEN}✅${NC} wasm32-wasi target installed"
else
    echo -e "${YELLOW}⚠️${NC}  wasm32-wasi target not installed"
    echo "   Installing now..."
    rustup target add wasm32-wasi
fi

if rustup target list --installed | grep -q wasm32-wasip1; then
    echo -e "${GREEN}✅${NC} wasm32-wasip1 target installed"
else
    echo -e "${YELLOW}⚠️${NC}  wasm32-wasip1 target not installed"
    echo "   Installing now..."
    rustup target add wasm32-wasip1
fi

echo ""
echo "3. Checking configuration files..."
echo "------------------------------"
check_file "spin.toml"
check_file "Dockerfile.spin"
check_file "k8s/namespace.yaml"
check_file "k8s/spinapp.yaml"
check_file "k8s/services.yaml"
check_file "k8s/ingress.yaml"
check_file "k8s/hpa.yaml"
check_file "k8s/secrets.yaml"
check_file "k8s/monitoring.yaml"

echo ""
echo "4. Checking services exist..."
echo "------------------------------"
SERVICES=(
    "authorworks-user-service"
    "authorworks-content-service"
    "authorworks-storage-service"
    "authorworks-editor-service"
    "authorworks-messaging-service"
    "authorworks-discovery-service"
    "authorworks-audio-service"
    "authorworks-video-service"
    "authorworks-graphics-service"
    "authorworks-subscription-service"
)

for service in "${SERVICES[@]}"; do
    if [ -d "$service" ]; then
        if [ -f "$service/Cargo.toml" ]; then
            echo -e "${GREEN}✅${NC} $service exists with Cargo.toml"
        else
            echo -e "${YELLOW}⚠️${NC}  $service exists but missing Cargo.toml"
            WARNINGS=$((WARNINGS + 1))
        fi
    else
        echo -e "${RED}❌${NC} $service directory not found"
        ERRORS=$((ERRORS + 1))
    fi
done

echo ""
echo "5. Checking Kubernetes connectivity..."
echo "------------------------------"
if kubectl cluster-info >/dev/null 2>&1; then
    CLUSTER=$(kubectl config current-context)
    echo -e "${GREEN}✅${NC} Connected to cluster: $CLUSTER"

    # Check for SpinKube CRDs
    if kubectl get crd spinapps.core.spinoperator.dev >/dev/null 2>&1; then
        echo -e "${GREEN}✅${NC} SpinKube CRDs installed"
    else
        echo -e "${YELLOW}⚠️${NC}  SpinKube CRDs not installed (will be installed during deployment)"
        WARNINGS=$((WARNINGS + 1))
    fi
else
    echo -e "${YELLOW}⚠️${NC}  Cannot connect to Kubernetes cluster"
    WARNINGS=$((WARNINGS + 1))
fi

echo ""
echo "6. Checking environment variables..."
echo "------------------------------"
check_env "DATABASE_URL" || echo "   Default: will use Kubernetes secrets"
check_env "REDIS_URL" || echo "   Default: will use Kubernetes secrets"
check_env "MINIO_ENDPOINT" || echo "   Default: http://minio:9000"
check_env "REGISTRY" || echo "   Default: ghcr.io/authorworks"

echo ""
echo "7. Checking container registry access..."
echo "------------------------------"
REGISTRY=${REGISTRY:-ghcr.io/authorworks}
if docker pull alpine:latest >/dev/null 2>&1; then
    echo -e "${GREEN}✅${NC} Docker daemon is accessible"
else
    echo -e "${RED}❌${NC} Cannot access Docker daemon"
    ERRORS=$((ERRORS + 1))
fi

echo ""
echo "8. Checking resource requirements..."
echo "------------------------------"
if [ -x "$(command -v kubectl)" ]; then
    # Check if we can get node resources
    if kubectl top nodes >/dev/null 2>&1; then
        TOTAL_CPU=$(kubectl get nodes -o json | jq '[.items[].status.capacity.cpu | tonumber] | add')
        TOTAL_MEM=$(kubectl get nodes -o json | jq '[.items[].status.capacity.memory | rtrimstr("Ki") | tonumber] | add / 1024 / 1024 | floor')

        echo -e "  Total cluster CPU: ${TOTAL_CPU} cores"
        echo -e "  Total cluster Memory: ${TOTAL_MEM} GB"

        # Minimum requirements for AuthorWorks
        MIN_CPU=8
        MIN_MEM=16

        if [ "$TOTAL_CPU" -ge "$MIN_CPU" ]; then
            echo -e "${GREEN}✅${NC} Sufficient CPU (minimum: ${MIN_CPU} cores)"
        else
            echo -e "${YELLOW}⚠️${NC}  Low CPU resources (recommended: ${MIN_CPU}+ cores)"
            WARNINGS=$((WARNINGS + 1))
        fi

        if [ "$TOTAL_MEM" -ge "$MIN_MEM" ]; then
            echo -e "${GREEN}✅${NC} Sufficient Memory (minimum: ${MIN_MEM} GB)"
        else
            echo -e "${YELLOW}⚠️${NC}  Low Memory (recommended: ${MIN_MEM}+ GB)"
            WARNINGS=$((WARNINGS + 1))
        fi
    else
        echo -e "${YELLOW}⚠️${NC}  Metrics server not available"
    fi
fi

echo ""
echo "9. Checking WASM build optimization tools..."
echo "------------------------------"
if command -v wasm-opt >/dev/null 2>&1; then
    echo -e "${GREEN}✅${NC} wasm-opt is installed"
else
    echo -e "${YELLOW}⚠️${NC}  wasm-opt not installed (optional for optimization)"
    echo "   Install with: npm install -g wasm-opt"
fi

if command -v wasmtime >/dev/null 2>&1; then
    echo -e "${GREEN}✅${NC} wasmtime is installed (for testing)"
else
    echo -e "${YELLOW}⚠️${NC}  wasmtime not installed (optional for testing)"
fi

echo ""
echo "=========================================="
echo "Verification Summary:"
echo "=========================================="

if [ $ERRORS -eq 0 ]; then
    if [ $WARNINGS -eq 0 ]; then
        echo -e "${GREEN}✅ All checks passed! Ready for deployment.${NC}"
        exit 0
    else
        echo -e "${YELLOW}⚠️  Verification completed with $WARNINGS warning(s).${NC}"
        echo "The deployment can proceed, but review warnings above."
        exit 0
    fi
else
    echo -e "${RED}❌ Verification failed with $ERRORS error(s) and $WARNINGS warning(s).${NC}"
    echo "Please fix the errors above before deploying."
    exit 1
fi