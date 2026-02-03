#!/bin/bash
#=============================================================================
# AuthorWorks K3d Deployment Script
#=============================================================================
#
# Deploys AuthorWorks to a K3d cluster that connects to external services
# running in Docker on the same homelab server (llm_network).
#
# External services expected on llm_network:
#   - PostgreSQL: neon-postgres-leopaska:5432
#   - Redis: redis-nd-leopaska:6379
#   - MinIO: minio-leopaska:9000
#   - Traefik: External reverse proxy
#
# Usage: ./scripts/deploy-k3d.sh [command]
#
# Commands:
#   create    - Create K3d cluster and deploy
#   deploy    - Deploy/update to existing cluster
#   build     - Build and push images to registry
#   status    - Show cluster status
#   logs      - Show service logs
#   destroy   - Delete K3d cluster
#   help      - Show this help
#
#=============================================================================

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CLUSTER_NAME="authorworks"
REGISTRY_NAME="authorworks-registry"
REGISTRY_PORT="5111"
K3D_CONFIG="$PROJECT_ROOT/k3d/cluster-config.yaml"
KUSTOMIZE_DIR="$PROJECT_ROOT/k3d"

cd "$PROJECT_ROOT"

# Utility functions
log_info() { echo -e "${BLUE}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_step() { echo -e "${CYAN}==>${NC} $*"; }

#=============================================================================
# Check Prerequisites
#=============================================================================
check_prerequisites() {
    log_step "Checking prerequisites..."
    
    local missing=()
    command -v docker &>/dev/null || missing+=("docker")
    command -v k3d &>/dev/null || missing+=("k3d")
    command -v kubectl &>/dev/null || missing+=("kubectl")
    command -v kustomize &>/dev/null || missing+=("kustomize (or use kubectl -k)")
    
    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing[*]}"
        echo ""
        echo "Install instructions:"
        echo "  k3d:      curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash"
        echo "  kubectl:  https://kubernetes.io/docs/tasks/tools/"
        echo "  kustomize: https://kubectl.docs.kubernetes.io/installation/kustomize/"
        exit 1
    fi
    
    # Check if llm_network exists
    if ! docker network inspect llm_network &>/dev/null; then
        log_warn "Docker network 'llm_network' not found. Creating it..."
        docker network create llm_network
    fi
    
    log_success "All prerequisites satisfied"
}

#=============================================================================
# Create Cluster
#=============================================================================
create_cluster() {
    log_step "Creating K3d cluster '$CLUSTER_NAME'..."
    
    # Check if cluster already exists
    if k3d cluster list | grep -q "^$CLUSTER_NAME"; then
        log_warn "Cluster '$CLUSTER_NAME' already exists"
        read -p "Delete and recreate? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            k3d cluster delete "$CLUSTER_NAME"
        else
            log_info "Using existing cluster"
            return 0
        fi
    fi
    
    # Create cluster with config
    if [[ -f "$K3D_CONFIG" ]]; then
        k3d cluster create --config "$K3D_CONFIG"
    else
        # Fallback: create cluster manually
        k3d cluster create "$CLUSTER_NAME" \
            --servers 1 \
            --agents 2 \
            --network llm_network \
            --port "8080:80@loadbalancer" \
            --port "8443:443@loadbalancer" \
            --registry-create "${REGISTRY_NAME}:0.0.0.0:${REGISTRY_PORT}" \
            --k3s-arg "--disable=traefik@server:*" \
            --wait
    fi
    
    log_success "Cluster created successfully"
    
    # Wait for nodes to be ready
    log_step "Waiting for nodes to be ready..."
    kubectl wait --for=condition=Ready nodes --all --timeout=120s
    
    log_success "All nodes are ready"
}

#=============================================================================
# Build Images
#=============================================================================
build_images() {
    log_step "Building Docker images..."
    
    local REGISTRY="localhost:${REGISTRY_PORT}"
    local services=(
        "user-service:services/user"
        "content-service:services/content"
        "storage-service:services/storage"
        "editor-service:services/editor"
        "subscription-service:services/subscription"
        "messaging-service:services/messaging"
        "discovery-service:services/discovery"
        "media-service:services/media"
        "content-worker:workers/content"
        "media-worker:workers/media"
    )
    
    for svc in "${services[@]}"; do
        local name="${svc%%:*}"
        local path="${svc##*:}"
        
        if [[ -f "$PROJECT_ROOT/$path/Dockerfile" ]]; then
            log_info "Building $name..."
            docker build -t "$REGISTRY/$name:latest" "$PROJECT_ROOT/$path"
            docker push "$REGISTRY/$name:latest"
            log_success "Built and pushed $name"
        else
            log_warn "No Dockerfile found for $name at $path"
        fi
    done
    
    log_success "All images built and pushed"
}

#=============================================================================
# Deploy Services
#=============================================================================
deploy_services() {
    log_step "Deploying AuthorWorks services..."
    
    # Create secrets from environment
    create_secrets
    
    # Apply Kustomize manifests
    if [[ -f "$KUSTOMIZE_DIR/kustomization.yaml" ]]; then
        kubectl apply -k "$KUSTOMIZE_DIR"
    else
        # Apply individual manifests
        for f in "$KUSTOMIZE_DIR"/*.yaml; do
            if [[ -f "$f" && "$f" != *"secrets.yaml" ]]; then
                kubectl apply -f "$f"
            fi
        done
    fi
    
    log_success "Manifests applied"
    
    # Wait for deployments
    log_step "Waiting for deployments to be ready..."
    kubectl wait --for=condition=Available deployment --all -n authorworks --timeout=300s || true
    
    log_success "Deployment complete"
}

#=============================================================================
# Create Secrets
#=============================================================================
create_secrets() {
    log_step "Creating secrets from environment..."
    
    # Load .env file if exists
    if [[ -f "$PROJECT_ROOT/.env" ]]; then
        set -a
        source "$PROJECT_ROOT/.env"
        set +a
    fi
    
    # Create namespace if not exists
    kubectl create namespace authorworks --dry-run=client -o yaml | kubectl apply -f -
    
    # Create secrets with environment variable substitution
    envsubst < "$KUSTOMIZE_DIR/secrets.yaml" | kubectl apply -f -
    
    log_success "Secrets created"
}

#=============================================================================
# Status
#=============================================================================
show_status() {
    log_step "Cluster Status"
    echo ""
    
    echo -e "${CYAN}=== Nodes ===${NC}"
    kubectl get nodes -o wide
    echo ""
    
    echo -e "${CYAN}=== Deployments ===${NC}"
    kubectl get deployments -n authorworks
    echo ""
    
    echo -e "${CYAN}=== Pods ===${NC}"
    kubectl get pods -n authorworks -o wide
    echo ""
    
    echo -e "${CYAN}=== Services ===${NC}"
    kubectl get services -n authorworks
    echo ""
    
    echo -e "${CYAN}=== Ingress ===${NC}"
    kubectl get ingress -n authorworks
    echo ""
    
    # Show access info
    echo -e "${GREEN}=== Access Info ===${NC}"
    echo "  API Gateway: http://localhost:8080"
    echo "  Auth (Logto): http://localhost:8080/api/logto/"
    echo ""
    echo "  If using external Traefik:"
    echo "  Main App: https://authorworks.leopaska.xyz"
    echo "  Auth: https://auth.authorworks.leopaska.xyz"
    echo "  Auth Admin: https://auth-admin.authorworks.leopaska.xyz"
}

#=============================================================================
# Logs
#=============================================================================
show_logs() {
    local service="${1:-}"
    
    if [[ -z "$service" ]]; then
        log_info "Available services:"
        kubectl get pods -n authorworks --no-headers -o custom-columns=":metadata.name"
        echo ""
        echo "Usage: $0 logs <service-name>"
        return
    fi
    
    log_step "Logs for $service"
    kubectl logs -n authorworks -l "app=$service" --tail=100 -f
}

#=============================================================================
# Destroy
#=============================================================================
destroy_cluster() {
    log_warn "This will delete the K3d cluster and all data!"
    read -p "Are you sure? [y/N] " -n 1 -r
    echo
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        log_step "Deleting cluster..."
        k3d cluster delete "$CLUSTER_NAME"
        log_success "Cluster deleted"
    else
        log_info "Cancelled"
    fi
}

#=============================================================================
# Help
#=============================================================================
show_help() {
    head -30 "$0" | tail -25
}

#=============================================================================
# Main
#=============================================================================
main() {
    local cmd="${1:-help}"
    shift || true
    
    case "$cmd" in
        create)
            check_prerequisites
            create_cluster
            build_images
            deploy_services
            show_status
            ;;
        deploy)
            check_prerequisites
            deploy_services
            show_status
            ;;
        build)
            check_prerequisites
            build_images
            ;;
        status)
            show_status
            ;;
        logs)
            show_logs "$@"
            ;;
        destroy)
            destroy_cluster
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            log_error "Unknown command: $cmd"
            show_help
            exit 1
            ;;
    esac
}

main "$@"

