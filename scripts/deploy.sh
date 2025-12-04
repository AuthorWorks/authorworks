#!/bin/bash
#=============================================================================
# AuthorWorks Unified Deployment Script
#=============================================================================
# 
# Usage: ./scripts/deploy.sh [environment] [options]
#
# Environments:
#   local       - Local development with Docker Compose
#   homelab     - K3s homelab deployment (Docker Compose + Traefik)
#   ec2         - AWS EC2 with Docker Compose (MVP production)
#   eks         - AWS EKS with Kubernetes (scalable production)
#
# Options:
#   --build     - Build services before deploying
#   --no-build  - Skip building (use existing images)
#   --down      - Tear down the deployment
#   --logs      - Follow logs after deployment
#   --verify    - Run health checks after deployment
#   --help      - Show this help message
#
# Examples:
#   ./scripts/deploy.sh local --build
#   ./scripts/deploy.sh homelab --verify
#   ./scripts/deploy.sh eks --build
#   ./scripts/deploy.sh local --down
#
#=============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Default values
ENVIRONMENT="${1:-local}"
BUILD=false
TEARDOWN=false
FOLLOW_LOGS=false
VERIFY=false

# Parse arguments
shift || true
while [[ $# -gt 0 ]]; do
    case $1 in
        --build) BUILD=true ;;
        --no-build) BUILD=false ;;
        --down) TEARDOWN=true ;;
        --logs) FOLLOW_LOGS=true ;;
        --verify) VERIFY=true ;;
        --help|-h)
            head -40 "$0" | tail -35
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
    shift
done

#=============================================================================
# Utility Functions
#=============================================================================

log_info() { echo -e "${BLUE}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_step() { echo -e "${CYAN}==>${NC} $*"; }

check_command() {
    if ! command -v "$1" &> /dev/null; then
        log_error "$1 is required but not installed."
        return 1
    fi
}

check_prerequisites() {
    log_step "Checking prerequisites..."
    local missing=()
    
    case "$ENVIRONMENT" in
        local|homelab|ec2)
            check_command docker || missing+=("docker")
            check_command docker-compose || docker compose version &>/dev/null || missing+=("docker-compose")
            ;;
        eks)
            check_command docker || missing+=("docker")
            check_command kubectl || missing+=("kubectl")
            check_command helm || missing+=("helm")
            check_command terraform || missing+=("terraform")
            check_command aws || missing+=("aws-cli")
            ;;
    esac
    
    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing[*]}"
        exit 1
    fi
    log_success "All prerequisites met"
}

load_env() {
    log_step "Loading environment configuration..."
    
    # Load .env if it exists
    if [[ -f "$PROJECT_ROOT/.env" ]]; then
        set -a
        source "$PROJECT_ROOT/.env"
        set +a
        log_info "Loaded .env file"
    else
        log_warn ".env file not found, using defaults"
    fi
    
    # Set defaults
    export DOMAIN="${DOMAIN:-localhost}"
    export DB_PASSWORD="${DB_PASSWORD:-authorworks123}"
    export JWT_SECRET="${JWT_SECRET:-authorworks-dev-secret-change-in-production}"
    export RABBITMQ_PASSWORD="${RABBITMQ_PASSWORD:-authorworks123}"
    export MINIO_ROOT_USER="${MINIO_ROOT_USER:-authorworks}"
    export MINIO_ROOT_PASSWORD="${MINIO_ROOT_PASSWORD:-authorworks123}"
    export GRAFANA_PASSWORD="${GRAFANA_PASSWORD:-authorworks123}"
}

#=============================================================================
# Build Functions
#=============================================================================

build_services() {
    log_step "Building services..."
    
    # Build frontend
    if [[ -d "$PROJECT_ROOT/frontend/landing/leptos-app" ]]; then
        log_info "Building Leptos frontend..."
        cd "$PROJECT_ROOT/frontend/landing/leptos-app"
        if command -v trunk &> /dev/null; then
            trunk build --release
        else
            log_warn "trunk not installed, skipping frontend build"
        fi
        cd "$PROJECT_ROOT"
    fi
    
    # Build Rust services for WASM
    log_info "Building Rust services..."
    if command -v rustup &> /dev/null; then
        rustup target add wasm32-wasi 2>/dev/null || true
    fi
    
    # Build with Docker Compose
    local compose_file
    case "$ENVIRONMENT" in
        local) compose_file="docker-compose.yml" ;;
        homelab) compose_file="docker-compose.homelab.yml" ;;
        ec2) compose_file="docker-compose.production.yml" ;;
    esac
    
    if [[ -n "${compose_file:-}" ]]; then
        log_info "Building Docker images..."
        docker compose -f "$compose_file" build
    fi
    
    log_success "Build complete"
}

#=============================================================================
# Deployment Functions
#=============================================================================

deploy_local() {
    local compose_file="docker-compose.yml"
    
    if [[ "$TEARDOWN" == true ]]; then
        log_step "Tearing down local deployment..."
        docker compose -f "$compose_file" down -v
        log_success "Local deployment torn down"
        return 0
    fi
    
    log_step "Deploying to local development environment..."
    
    [[ "$BUILD" == true ]] && build_services
    
    # Start services
    docker compose -f "$compose_file" up -d
    
    log_success "Local deployment complete!"
    echo ""
    log_info "Services available at:"
    echo "  - App:         http://localhost:8080"
    echo "  - Logto:       http://localhost:3001"
    echo "  - Logto Admin: http://localhost:3002"
    echo "  - Grafana:     http://localhost:3000"
    echo "  - RabbitMQ:    http://localhost:15672"
    echo "  - MinIO:       http://localhost:9001"
    echo "  - Mailpit:     http://localhost:8025"
}

deploy_homelab() {
    local compose_file="docker-compose.homelab.yml"
    
    if [[ "$TEARDOWN" == true ]]; then
        log_step "Tearing down homelab deployment..."
        docker compose -f "$compose_file" down
        log_success "Homelab deployment torn down"
        return 0
    fi
    
    log_step "Deploying to homelab K3s environment..."
    
    # Verify required env vars
    if [[ -z "${DOMAIN:-}" || "$DOMAIN" == "localhost" ]]; then
        log_error "DOMAIN must be set for homelab deployment (e.g., leopaska.xyz)"
        exit 1
    fi
    
    [[ "$BUILD" == true ]] && build_services
    
    # Start services
    docker compose -f "$compose_file" up -d
    
    log_success "Homelab deployment complete!"
    echo ""
    log_info "Services available at:"
    echo "  - App:         https://authorworks.$DOMAIN"
    echo "  - Auth:        https://auth.authorworks.$DOMAIN"
    echo "  - Auth Admin:  https://auth-admin.authorworks.$DOMAIN"
}

deploy_ec2() {
    local compose_file="docker-compose.production.yml"
    
    if [[ "$TEARDOWN" == true ]]; then
        log_step "Tearing down EC2 deployment..."
        docker compose -f "$compose_file" down
        log_success "EC2 deployment torn down"
        return 0
    fi
    
    log_step "Deploying to EC2 production environment..."
    
    # Verify required env vars
    local required_vars=(DATABASE_URL REDIS_URL DOMAIN JWT_SECRET)
    for var in "${required_vars[@]}"; do
        if [[ -z "${!var:-}" ]]; then
            log_error "Required variable $var is not set"
            exit 1
        fi
    done
    
    [[ "$BUILD" == true ]] && build_services
    
    # Setup SSL if needed
    if [[ ! -d "/etc/letsencrypt/live/$DOMAIN" ]]; then
        log_warn "SSL certificates not found. Run certbot manually:"
        echo "  certbot certonly --standalone -d $DOMAIN -d auth.$DOMAIN"
    fi
    
    # Start services
    docker compose -f "$compose_file" up -d
    
    log_success "EC2 deployment complete!"
    echo ""
    log_info "Services available at:"
    echo "  - App:  https://$DOMAIN"
    echo "  - Auth: https://auth.$DOMAIN"
}

deploy_eks() {
    if [[ "$TEARDOWN" == true ]]; then
        log_step "Tearing down EKS deployment..."
        kubectl delete namespace authorworks --ignore-not-found
        log_success "EKS deployment torn down"
        return 0
    fi
    
    log_step "Deploying to AWS EKS..."
    
    # Verify AWS credentials
    if ! aws sts get-caller-identity &>/dev/null; then
        log_error "AWS credentials not configured. Run 'aws configure' first."
        exit 1
    fi
    
    local aws_region="${AWS_REGION:-us-west-2}"
    
    # Initialize and apply Terraform
    log_info "Provisioning infrastructure with Terraform..."
    cd "$PROJECT_ROOT/terraform/aws"
    
    if [[ ! -f "terraform.tfvars" ]]; then
        log_error "terraform.tfvars not found. Copy terraform.tfvars.example and configure it."
        exit 1
    fi
    
    terraform init
    terraform plan -out=tfplan
    
    read -p "Apply Terraform changes? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        terraform apply tfplan
    else
        log_warn "Terraform apply skipped"
        cd "$PROJECT_ROOT"
        return 0
    fi
    
    # Get outputs
    local cluster_name=$(terraform output -raw cluster_name)
    local ecr_repo=$(terraform output -raw ecr_repository_url)
    
    cd "$PROJECT_ROOT"
    
    # Configure kubectl
    log_info "Configuring kubectl..."
    aws eks update-kubeconfig --region "$aws_region" --name "$cluster_name"
    
    # Build and push
    if [[ "$BUILD" == true ]]; then
        log_info "Building and pushing Docker images..."
        aws ecr get-login-password --region "$aws_region" | docker login --username AWS --password-stdin "$ecr_repo"
        docker build -f Dockerfile.spin -t "$ecr_repo:latest" .
        docker push "$ecr_repo:latest"
    fi
    
    # Install SpinKube operator
    log_info "Installing SpinKube operator..."
    helm repo add spinkube https://spinkube.github.io/charts 2>/dev/null || true
    helm repo update
    helm upgrade --install spin-operator spinkube/spin-operator \
        --namespace spin-system --create-namespace --version 0.2.0
    
    # Deploy application
    log_info "Deploying AuthorWorks..."
    kubectl apply -k k8s/overlays/production
    
    # Wait for deployment
    log_info "Waiting for deployment to be ready..."
    kubectl wait --for=condition=ready --timeout=600s \
        spinapp/authorworks-platform -n authorworks || true
    
    log_success "EKS deployment complete!"
    echo ""
    log_info "Application status:"
    kubectl get spinapp -n authorworks
    echo ""
    kubectl get ingress -n authorworks
}

#=============================================================================
# Verification Functions
#=============================================================================

verify_deployment() {
    log_step "Verifying deployment..."
    
    local endpoints=()
    local base_url
    
    case "$ENVIRONMENT" in
        local)
            base_url="http://localhost:8080"
            endpoints=(
                "$base_url/health"
                "http://localhost:3001/api/status"
            )
            ;;
        homelab)
            base_url="https://authorworks.$DOMAIN"
            endpoints=(
                "$base_url/health"
                "https://auth.authorworks.$DOMAIN/api/status"
            )
            ;;
        ec2)
            base_url="https://$DOMAIN"
            endpoints=(
                "$base_url/health"
                "https://auth.$DOMAIN/api/status"
            )
            ;;
        eks)
            log_info "Checking Kubernetes resources..."
            kubectl get pods -n authorworks
            kubectl get services -n authorworks
            return 0
            ;;
    esac
    
    echo ""
    for endpoint in "${endpoints[@]}"; do
        if curl -fsS --max-time 5 "$endpoint" &>/dev/null; then
            log_success "$endpoint - OK"
        else
            log_warn "$endpoint - FAILED"
        fi
    done
}

follow_logs() {
    log_step "Following logs..."
    
    case "$ENVIRONMENT" in
        local)
            docker compose -f docker-compose.yml logs -f
            ;;
        homelab)
            docker compose -f docker-compose.homelab.yml logs -f
            ;;
        ec2)
            docker compose -f docker-compose.production.yml logs -f
            ;;
        eks)
            kubectl logs -n authorworks -l app=authorworks --tail=100 -f
            ;;
    esac
}

#=============================================================================
# Main
#=============================================================================

main() {
    echo ""
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║          AuthorWorks Deployment Script                    ║${NC}"
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    log_info "Environment: $ENVIRONMENT"
    log_info "Build: $BUILD"
    log_info "Teardown: $TEARDOWN"
    echo ""
    
    check_prerequisites
    load_env
    
    case "$ENVIRONMENT" in
        local)
            deploy_local
            ;;
        homelab)
            deploy_homelab
            ;;
        ec2)
            deploy_ec2
            ;;
        eks)
            deploy_eks
            ;;
        *)
            log_error "Unknown environment: $ENVIRONMENT"
            log_info "Valid environments: local, homelab, ec2, eks"
            exit 1
            ;;
    esac
    
    [[ "$VERIFY" == true ]] && verify_deployment
    [[ "$FOLLOW_LOGS" == true ]] && follow_logs
    
    echo ""
    log_success "Deployment script complete!"
}

main "$@"

