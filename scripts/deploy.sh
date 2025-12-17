#!/bin/bash
#=============================================================================
# AuthorWorks Unified Deployment Script
#=============================================================================
#
# Usage: ./scripts/deploy.sh [environment] [options]
#
# Environments:
#   local       - Full stack Docker Compose (includes all infrastructure)
#   dev         - Development with external services (localist network)
#   homelab     - Homelab K3s/Docker with Traefik (llm_network)
#   k3d         - Local K3d Kubernetes cluster
#   ec2         - AWS EC2 with Docker Compose (MVP production)
#   eks         - AWS EKS with Kubernetes (scalable production)
#
# Options:
#   --build     - Build services before deploying
#   --no-build  - Skip building (use existing images)
#   --down      - Tear down the deployment
#   --logs      - Follow logs after deployment
#   --verify    - Run health checks after deployment
#   --status    - Show deployment status
#   --init      - Initialize environment (create .env, setup)
#   --help      - Show this help message
#
# Examples:
#   ./scripts/deploy.sh local --build
#   ./scripts/deploy.sh dev --verify
#   ./scripts/deploy.sh homelab --build --verify
#   ./scripts/deploy.sh k3d create
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
MAGENTA='\033[0;35m'
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
SHOW_STATUS=false
INIT=false
K3D_COMMAND=""

# Parse arguments
shift || true
while [[ $# -gt 0 ]]; do
    case $1 in
        --build) BUILD=true ;;
        --no-build) BUILD=false ;;
        --down) TEARDOWN=true ;;
        --logs) FOLLOW_LOGS=true ;;
        --verify) VERIFY=true ;;
        --status) SHOW_STATUS=true ;;
        --init) INIT=true ;;
        --help|-h)
            head -45 "$0" | tail -40
            exit 0
            ;;
        # K3d subcommands
        create|deploy|build|status|logs|destroy)
            K3D_COMMAND="$1"
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help for usage information"
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
log_header() { echo -e "\n${MAGENTA}━━━ $* ━━━${NC}\n"; }

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
        local|dev|homelab|ec2)
            check_command docker || missing+=("docker")
            docker compose version &>/dev/null || check_command docker-compose || missing+=("docker-compose")
            ;;
        k3d)
            check_command docker || missing+=("docker")
            check_command k3d || missing+=("k3d")
            check_command kubectl || missing+=("kubectl")
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
        echo ""
        echo "Installation hints:"
        for tool in "${missing[@]}"; do
            case "$tool" in
                docker) echo "  docker: https://docs.docker.com/get-docker/" ;;
                k3d) echo "  k3d: curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash" ;;
                kubectl) echo "  kubectl: https://kubernetes.io/docs/tasks/tools/" ;;
                helm) echo "  helm: https://helm.sh/docs/intro/install/" ;;
                terraform) echo "  terraform: https://www.terraform.io/downloads" ;;
                aws-cli) echo "  aws-cli: https://aws.amazon.com/cli/" ;;
            esac
        done
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
        log_warn ".env file not found"
        if [[ "$INIT" == true ]]; then
            log_info "Creating .env from .env.example..."
            if [[ -f "$PROJECT_ROOT/.env.example" ]]; then
                cp "$PROJECT_ROOT/.env.example" "$PROJECT_ROOT/.env"
                log_success "Created .env file - please configure it"
            else
                log_error ".env.example not found"
                exit 1
            fi
        else
            log_warn "Run with --init to create .env from template"
        fi
    fi

    # Set defaults based on environment
    case "$ENVIRONMENT" in
        local)
            export DOMAIN="${DOMAIN:-localhost}"
            export DB_PASSWORD="${DB_PASSWORD:-authorworks123}"
            export RABBITMQ_PASSWORD="${RABBITMQ_PASSWORD:-authorworks123}"
            export MINIO_ROOT_USER="${MINIO_ROOT_USER:-authorworks}"
            export MINIO_ROOT_PASSWORD="${MINIO_ROOT_PASSWORD:-authorworks123}"
            ;;
        dev)
            export DOMAIN="${DOMAIN:-localhost}"
            export POSTGRES_HOST="${DEV_POSTGRES_HOST:-localist-postgres-local}"
            export REDIS_HOST="${DEV_REDIS_HOST:-localist-redis-local}"
            export MINIO_HOST="${DEV_MINIO_HOST:-localist-minio-local}"
            ;;
        homelab)
            export DOMAIN="${DOMAIN:-leopaska.xyz}"
            export POSTGRES_HOST="${HOMELAB_POSTGRES_HOST:-neon-postgres-leopaska}"
            export REDIS_HOST="${HOMELAB_REDIS_HOST:-redis-nd-leopaska}"
            export MINIO_HOST="${HOMELAB_MINIO_HOST:-minio-leopaska}"
            ;;
        k3d)
            export K3D_CLUSTER_NAME="${K3D_CLUSTER_NAME:-authorworks}"
            export K3D_REGISTRY_PORT="${K3D_REGISTRY_PORT:-5111}"
            ;;
        ec2|eks)
            # Production requires explicit configuration
            if [[ -z "${DOMAIN:-}" || "$DOMAIN" == "localhost" ]]; then
                log_error "DOMAIN must be set for $ENVIRONMENT deployment"
                exit 1
            fi
            ;;
    esac

    # Common defaults
    export JWT_SECRET="${JWT_SECRET:-authorworks-dev-secret-change-in-production}"
    export GRAFANA_PASSWORD="${GRAFANA_PASSWORD:-authorworks123}"
}

validate_env() {
    log_step "Validating environment configuration..."
    local errors=0

    case "$ENVIRONMENT" in
        ec2|eks)
            [[ -z "${DATABASE_URL:-}" ]] && { log_error "DATABASE_URL is required"; ((errors++)); }
            [[ -z "${REDIS_URL:-}" ]] && { log_error "REDIS_URL is required"; ((errors++)); }
            [[ -z "${JWT_SECRET:-}" || "${JWT_SECRET}" == *"dev-secret"* ]] && { log_warn "JWT_SECRET should be changed for production"; }
            ;;
        homelab)
            [[ -z "${POSTGRES_PASSWORD:-}" ]] && { log_error "POSTGRES_PASSWORD is required"; ((errors++)); }
            ;;
    esac

    if [[ $errors -gt 0 ]]; then
        log_error "Environment validation failed with $errors error(s)"
        exit 1
    fi
    log_success "Environment validated"
}

#=============================================================================
# Docker Compose Helper
#=============================================================================

docker_compose() {
    if docker compose version &>/dev/null; then
        docker compose "$@"
    else
        docker-compose "$@"
    fi
}

#=============================================================================
# Build Functions
#=============================================================================

build_services() {
    log_step "Building services..."

    # Build frontend if exists
    if [[ -d "$PROJECT_ROOT/frontend/app" ]]; then
        log_info "Building Next.js frontend..."
        if [[ -f "$PROJECT_ROOT/frontend/app/Dockerfile" ]]; then
            docker build -t authorworks-frontend:latest "$PROJECT_ROOT/frontend/app"
        fi
    fi

    # Build with Docker Compose
    local compose_file
    case "$ENVIRONMENT" in
        local) compose_file="docker-compose.yml" ;;
        dev) compose_file="docker-compose.local.yml" ;;
        homelab) compose_file="docker-compose.homelab.yml" ;;
        ec2) compose_file="docker-compose.production.yml" ;;
    esac

    if [[ -n "${compose_file:-}" && -f "$compose_file" ]]; then
        log_info "Building Docker images with $compose_file..."
        docker_compose -f "$compose_file" build
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
        docker_compose -f "$compose_file" down -v
        log_success "Local deployment torn down"
        return 0
    fi

    if [[ "$SHOW_STATUS" == true ]]; then
        docker_compose -f "$compose_file" ps
        return 0
    fi

    log_step "Deploying to local development environment..."

    [[ "$BUILD" == true ]] && build_services

    # Start services
    docker_compose -f "$compose_file" up -d

    log_success "Local deployment complete!"
    echo ""
    log_info "Services available at:"
    echo "  - App:         http://localhost:8080"
    echo "  - Frontend:    http://localhost:3010"
    echo "  - Logto:       http://localhost:3001"
    echo "  - Logto Admin: http://localhost:3002"
    echo "  - Grafana:     http://localhost:3000"
    echo "  - RabbitMQ:    http://localhost:15672"
    echo "  - MinIO:       http://localhost:9001"
    echo "  - Mailpit:     http://localhost:8025"
}

deploy_dev() {
    local compose_file="docker-compose.local.yml"

    if [[ ! -f "$compose_file" ]]; then
        log_error "docker-compose.local.yml not found"
        exit 1
    fi

    # Check if external network exists
    local network="${DEV_NETWORK:-localist_localist-local}"
    if ! docker network inspect "$network" &>/dev/null; then
        log_error "Docker network '$network' not found"
        log_info "Make sure external services are running (PostgreSQL, Redis, MinIO)"
        exit 1
    fi

    if [[ "$TEARDOWN" == true ]]; then
        log_step "Tearing down dev deployment..."
        docker_compose -f "$compose_file" down
        log_success "Dev deployment torn down"
        return 0
    fi

    if [[ "$SHOW_STATUS" == true ]]; then
        docker_compose -f "$compose_file" ps
        return 0
    fi

    log_step "Deploying to dev environment (using external services)..."

    [[ "$BUILD" == true ]] && build_services

    docker_compose -f "$compose_file" up -d

    log_success "Dev deployment complete!"
    echo ""
    log_info "Services available at:"
    echo "  - API Gateway: http://localhost:8080"
    echo "  - Frontend:    http://localhost:3010"
    echo "  - Logto:       http://localhost:3012"
    echo "  - Logto Admin: http://localhost:3013"
    echo "  - RabbitMQ:    http://localhost:15672"
}

deploy_homelab() {
    local compose_file="docker-compose.homelab.yml"

    if [[ ! -f "$compose_file" ]]; then
        log_error "docker-compose.homelab.yml not found"
        exit 1
    fi

    # Check if external network exists
    local network="${HOMELAB_NETWORK:-llm_network}"
    if ! docker network inspect "$network" &>/dev/null; then
        log_warn "Docker network '$network' not found. Creating it..."
        docker network create "$network"
    fi

    if [[ "$TEARDOWN" == true ]]; then
        log_step "Tearing down homelab deployment..."
        docker_compose -f "$compose_file" down
        log_success "Homelab deployment torn down"
        return 0
    fi

    if [[ "$SHOW_STATUS" == true ]]; then
        docker_compose -f "$compose_file" ps
        return 0
    fi

    log_step "Deploying to homelab environment..."

    # Verify domain is set
    if [[ -z "${DOMAIN:-}" || "$DOMAIN" == "localhost" ]]; then
        log_error "DOMAIN must be set for homelab deployment (e.g., leopaska.xyz)"
        exit 1
    fi

    [[ "$BUILD" == true ]] && build_services

    docker_compose -f "$compose_file" up -d

    log_success "Homelab deployment complete!"
    echo ""
    log_info "Services available at:"
    echo "  - App:         https://authorworks.$DOMAIN"
    echo "  - Auth:        https://auth.authorworks.$DOMAIN"
    echo "  - Auth Admin:  https://auth-admin.authorworks.$DOMAIN"
}

deploy_k3d() {
    local cmd="${K3D_COMMAND:-deploy}"

    case "$cmd" in
        create)
            log_step "Creating K3d cluster..."
            if [[ -f "$PROJECT_ROOT/k3d/cluster-config.yaml" ]]; then
                k3d cluster create --config "$PROJECT_ROOT/k3d/cluster-config.yaml"
            else
                k3d cluster create "${K3D_CLUSTER_NAME:-authorworks}" \
                    --servers 1 \
                    --agents 2 \
                    --port "8080:80@loadbalancer" \
                    --port "8443:443@loadbalancer" \
                    --registry-create "authorworks-registry:0.0.0.0:${K3D_REGISTRY_PORT:-5111}" \
                    --wait
            fi
            log_success "K3d cluster created"

            # Deploy after creation
            deploy_k3d_services
            ;;
        deploy)
            deploy_k3d_services
            ;;
        build)
            build_k3d_images
            ;;
        status)
            kubectl get all -n authorworks 2>/dev/null || log_warn "Namespace 'authorworks' not found"
            ;;
        destroy)
            log_step "Destroying K3d cluster..."
            k3d cluster delete "${K3D_CLUSTER_NAME:-authorworks}"
            log_success "K3d cluster destroyed"
            ;;
        *)
            if [[ "$TEARDOWN" == true ]]; then
                k3d cluster delete "${K3D_CLUSTER_NAME:-authorworks}"
            elif [[ "$SHOW_STATUS" == true ]]; then
                kubectl get all -n authorworks 2>/dev/null || echo "Namespace not found"
            else
                deploy_k3d_services
            fi
            ;;
    esac
}

build_k3d_images() {
    log_step "Building images for K3d..."
    local registry="localhost:${K3D_REGISTRY_PORT:-5111}"

    # Build worker images
    for worker in content media; do
        if [[ -f "$PROJECT_ROOT/workers/$worker/Dockerfile" ]]; then
            log_info "Building $worker-worker..."
            docker build -t "$registry/$worker-worker:latest" "$PROJECT_ROOT/workers/$worker"
            docker push "$registry/$worker-worker:latest"
        fi
    done

    log_success "Images built and pushed to local registry"
}

deploy_k3d_services() {
    log_step "Deploying services to K3d..."

    # Create namespace
    kubectl create namespace authorworks --dry-run=client -o yaml | kubectl apply -f -

    # Apply manifests
    if [[ -f "$PROJECT_ROOT/k3d/kustomization.yaml" ]]; then
        # Load env vars and substitute
        if [[ -f "$PROJECT_ROOT/.env" ]]; then
            set -a; source "$PROJECT_ROOT/.env"; set +a
        fi
        envsubst < "$PROJECT_ROOT/k3d/secrets.yaml" | kubectl apply -f - 2>/dev/null || true
        kubectl apply -k "$PROJECT_ROOT/k3d"
    else
        for f in "$PROJECT_ROOT/k3d"/*.yaml; do
            [[ -f "$f" ]] && kubectl apply -f "$f"
        done
    fi

    log_success "K3d deployment complete!"
    echo ""
    kubectl get pods -n authorworks
}

deploy_ec2() {
    local compose_file="docker-compose.production.yml"

    if [[ ! -f "$compose_file" ]]; then
        log_error "docker-compose.production.yml not found"
        exit 1
    fi

    if [[ "$TEARDOWN" == true ]]; then
        log_step "Tearing down EC2 deployment..."
        docker_compose -f "$compose_file" down
        log_success "EC2 deployment torn down"
        return 0
    fi

    if [[ "$SHOW_STATUS" == true ]]; then
        docker_compose -f "$compose_file" ps
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

    docker_compose -f "$compose_file" up -d

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

    if [[ "$SHOW_STATUS" == true ]]; then
        kubectl get all -n authorworks
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
    cd "$PROJECT_ROOT/terraform"

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
    local cluster_name=$(terraform output -raw cluster_name 2>/dev/null || echo "authorworks-production")
    local ecr_repo=$(terraform output -raw ecr_repository_url 2>/dev/null || echo "")

    cd "$PROJECT_ROOT"

    # Configure kubectl
    log_info "Configuring kubectl..."
    aws eks update-kubeconfig --region "$aws_region" --name "$cluster_name"

    # Build and push
    if [[ "$BUILD" == true && -n "$ecr_repo" ]]; then
        log_info "Building and pushing Docker images..."
        aws ecr get-login-password --region "$aws_region" | docker login --username AWS --password-stdin "$ecr_repo"
        docker build -f Dockerfile.production -t "$ecr_repo:latest" .
        docker push "$ecr_repo:latest"
    fi

    # Install SpinKube operator if needed
    if ! kubectl get crd spinapps.core.spinoperator.dev &>/dev/null; then
        log_info "Installing SpinKube operator..."
        helm repo add spinkube https://spinkube.github.io/charts 2>/dev/null || true
        helm repo update
        helm upgrade --install spin-operator spinkube/spin-operator \
            --namespace spin-system --create-namespace --version 0.2.0
    fi

    # Deploy application
    log_info "Deploying AuthorWorks..."
    kubectl apply -k k8s/overlays/production

    # Wait for deployment
    log_info "Waiting for deployment to be ready..."
    kubectl wait --for=condition=Available deployment --all -n authorworks --timeout=600s || true

    log_success "EKS deployment complete!"
    echo ""
    log_info "Application status:"
    kubectl get pods -n authorworks
    echo ""
    kubectl get ingress -n authorworks 2>/dev/null || true
}

#=============================================================================
# Verification Functions
#=============================================================================

verify_deployment() {
    log_step "Verifying deployment..."

    local endpoints=()
    local base_url
    local wait_time=5
    local max_retries=3

    case "$ENVIRONMENT" in
        local)
            base_url="http://localhost:8080"
            endpoints=(
                "$base_url/health:API Gateway"
                "http://localhost:3001/api/status:Logto"
            )
            ;;
        dev)
            base_url="http://localhost:8080"
            endpoints=(
                "$base_url/health:API Gateway"
                "http://localhost:3012/api/status:Logto"
            )
            ;;
        homelab)
            base_url="https://authorworks.$DOMAIN"
            endpoints=(
                "$base_url/health:API Gateway"
                "https://auth.authorworks.$DOMAIN/api/status:Logto"
            )
            ;;
        ec2)
            base_url="https://$DOMAIN"
            endpoints=(
                "$base_url/health:API Gateway"
                "https://auth.$DOMAIN/api/status:Logto"
            )
            ;;
        k3d|eks)
            log_info "Checking Kubernetes resources..."
            echo ""
            kubectl get pods -n authorworks
            echo ""
            kubectl get services -n authorworks
            return 0
            ;;
    esac

    log_info "Waiting ${wait_time}s for services to start..."
    sleep "$wait_time"

    echo ""
    local failed=0
    for entry in "${endpoints[@]}"; do
        local endpoint="${entry%%:*}"
        local name="${entry##*:}"

        local success=false
        for ((i=1; i<=max_retries; i++)); do
            if curl -fsS --max-time 10 "$endpoint" &>/dev/null; then
                success=true
                break
            fi
            [[ $i -lt $max_retries ]] && sleep 2
        done

        if $success; then
            log_success "$name - OK"
        else
            log_error "$name - FAILED ($endpoint)"
            ((failed++))
        fi
    done

    if [[ $failed -gt 0 ]]; then
        log_warn "$failed service(s) failed health check"
        return 1
    fi
    log_success "All services healthy!"
}

follow_logs() {
    log_step "Following logs..."

    case "$ENVIRONMENT" in
        local)
            docker_compose -f docker-compose.yml logs -f
            ;;
        dev)
            docker_compose -f docker-compose.local.yml logs -f
            ;;
        homelab)
            docker_compose -f docker-compose.homelab.yml logs -f
            ;;
        ec2)
            docker_compose -f docker-compose.production.yml logs -f
            ;;
        k3d|eks)
            kubectl logs -n authorworks -l app.kubernetes.io/part-of=authorworks --tail=100 -f
            ;;
    esac
}

#=============================================================================
# Main
#=============================================================================

main() {
    echo ""
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║              AuthorWorks Deployment Script                    ║${NC}"
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    log_info "Environment: $ENVIRONMENT"
    log_info "Build: $BUILD | Teardown: $TEARDOWN | Verify: $VERIFY"
    echo ""

    check_prerequisites
    load_env

    if [[ "$INIT" == true && ! "$TEARDOWN" == true ]]; then
        log_success "Environment initialized. Edit .env and run again."
        exit 0
    fi

    validate_env

    case "$ENVIRONMENT" in
        local)
            deploy_local
            ;;
        dev)
            deploy_dev
            ;;
        homelab)
            deploy_homelab
            ;;
        k3d)
            deploy_k3d
            ;;
        ec2)
            deploy_ec2
            ;;
        eks)
            deploy_eks
            ;;
        *)
            log_error "Unknown environment: $ENVIRONMENT"
            log_info "Valid environments: local, dev, homelab, k3d, ec2, eks"
            exit 1
            ;;
    esac

    [[ "$VERIFY" == true ]] && verify_deployment
    [[ "$FOLLOW_LOGS" == true ]] && follow_logs

    echo ""
    log_success "Deployment script complete!"
}

main "$@"
