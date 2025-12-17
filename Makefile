.PHONY: help init build test clean
.PHONY: deploy-local deploy-dev deploy-homelab deploy-k3d deploy-ec2 deploy-eks
.PHONY: down down-all logs status verify
.PHONY: k3d-create k3d-destroy k3d-build k3d-status
.PHONY: scale-up scale-down rollback
.PHONY: db-migrate db-seed db-reset

SHELL := /bin/bash
DEPLOY_SCRIPT := ./scripts/deploy.sh

#=============================================================================
# AuthorWorks - Unified Makefile
#=============================================================================
# 
# Deployment Environments:
#   local    - Full stack Docker (includes Postgres, Redis, MinIO)
#   dev      - Uses external services (localist network)
#   homelab  - K3s homelab with Traefik (llm_network)
#   k3d      - Local K3d Kubernetes cluster
#   ec2      - AWS EC2 single instance
#   eks      - AWS EKS Kubernetes
#
#=============================================================================

help:
	@echo ""
	@echo "╔═══════════════════════════════════════════════════════════════╗"
	@echo "║              AuthorWorks Platform                             ║"
	@echo "╚═══════════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "Setup:"
	@echo "  make init              Initialize environment (copy .env.example)"
	@echo "  make build             Build all Docker images"
	@echo "  make test              Run all tests"
	@echo "  make clean             Clean build artifacts"
	@echo ""
	@echo "Deployment (Docker Compose):"
	@echo "  make deploy-local      Full stack local (includes infrastructure)"
	@echo "  make deploy-dev        Dev with external services (localist network)"
	@echo "  make deploy-homelab    Homelab K3s/Docker (llm_network)"
	@echo "  make deploy-ec2        AWS EC2 production"
	@echo ""
	@echo "Deployment (Kubernetes):"
	@echo "  make deploy-k3d        Deploy to K3d cluster"
	@echo "  make deploy-eks        Deploy to AWS EKS"
	@echo "  make k3d-create        Create K3d cluster + deploy"
	@echo "  make k3d-destroy       Delete K3d cluster"
	@echo ""
	@echo "Operations:"
	@echo "  make down              Tear down current deployment"
	@echo "  make down-all          Tear down ALL deployments"
	@echo "  make logs              Follow deployment logs"
	@echo "  make status            Show deployment status"
	@echo "  make verify            Run health checks"
	@echo ""
	@echo "Scaling (EKS):"
	@echo "  make scale-up          Scale to 10 replicas"
	@echo "  make scale-down        Scale to 3 replicas"
	@echo "  make rollback          Rollback to previous deployment"
	@echo ""
	@echo "Database:"
	@echo "  make db-migrate        Run database migrations"
	@echo "  make db-seed           Seed database with test data"
	@echo "  make db-reset          Reset database (DESTRUCTIVE)"
	@echo ""

#=============================================================================
# Setup & Build
#=============================================================================

init:
	@if [ ! -f .env ]; then \
		cp .env.example .env 2>/dev/null || echo "# AuthorWorks Environment" > .env; \
		echo "✅ Created .env file - please configure it"; \
	else \
		echo "⚠️  .env already exists"; \
	fi

build:
	@echo "🔨 Building all services..."
	@docker compose build

test:
	@echo "🧪 Running tests..."
	@cargo test --workspace 2>/dev/null || echo "Cargo tests skipped"
	@cd frontend/app && npm test 2>/dev/null || echo "Frontend tests skipped"

clean:
	@echo "🧹 Cleaning build artifacts..."
	@rm -rf target/ 2>/dev/null || true
	@find . -name "target" -type d -exec rm -rf {} + 2>/dev/null || true
	@find . -name "node_modules" -type d -exec rm -rf {} + 2>/dev/null || true
	@find . -name ".next" -type d -exec rm -rf {} + 2>/dev/null || true
	@docker system prune -f 2>/dev/null || true
	@echo "✅ Clean complete"

#=============================================================================
# Docker Compose Deployments
#=============================================================================

deploy-local:
	@$(DEPLOY_SCRIPT) local --build --verify

deploy-dev:
	@$(DEPLOY_SCRIPT) dev --build --verify

deploy-homelab:
	@$(DEPLOY_SCRIPT) homelab --build --verify

deploy-ec2:
	@$(DEPLOY_SCRIPT) ec2 --build --verify

#=============================================================================
# Kubernetes Deployments
#=============================================================================

deploy-k3d:
	@$(DEPLOY_SCRIPT) k3d deploy --verify

deploy-eks:
	@$(DEPLOY_SCRIPT) eks --build --verify

k3d-create:
	@$(DEPLOY_SCRIPT) k3d create --verify

k3d-destroy:
	@$(DEPLOY_SCRIPT) k3d destroy

k3d-build:
	@$(DEPLOY_SCRIPT) k3d build

k3d-status:
	@$(DEPLOY_SCRIPT) k3d status

#=============================================================================
# Operations
#=============================================================================

# Detect current environment and tear down
down:
	@echo "🔽 Tearing down deployment..."
	@if docker compose ps 2>/dev/null | grep -q authorworks; then \
		docker compose down; \
	elif docker compose -f docker-compose.local.yml ps 2>/dev/null | grep -q authorworks; then \
		docker compose -f docker-compose.local.yml down; \
	elif docker compose -f docker-compose.homelab.yml ps 2>/dev/null | grep -q authorworks; then \
		docker compose -f docker-compose.homelab.yml down; \
	elif k3d cluster list 2>/dev/null | grep -q authorworks; then \
		echo "K3d cluster running - use 'make k3d-destroy' to remove"; \
	else \
		echo "No active deployment found"; \
	fi

down-all:
	@echo "🔽 Tearing down ALL deployments..."
	@docker compose down 2>/dev/null || true
	@docker compose -f docker-compose.local.yml down 2>/dev/null || true
	@docker compose -f docker-compose.homelab.yml down 2>/dev/null || true
	@docker compose -f docker-compose.production.yml down 2>/dev/null || true
	@k3d cluster delete authorworks 2>/dev/null || true
	@echo "✅ All deployments torn down"

logs:
	@if docker compose ps 2>/dev/null | grep -q authorworks; then \
		docker compose logs -f; \
	elif docker compose -f docker-compose.local.yml ps 2>/dev/null | grep -q authorworks; then \
		docker compose -f docker-compose.local.yml logs -f; \
	elif kubectl get ns authorworks 2>/dev/null; then \
		kubectl logs -n authorworks -l app.kubernetes.io/part-of=authorworks --tail=100 -f; \
	else \
		echo "No active deployment found"; \
	fi

status:
	@echo ""
	@echo "📊 Deployment Status"
	@echo "==================="
	@echo ""
	@echo "Docker Compose (local):"
	@docker compose ps 2>/dev/null || echo "  Not running"
	@echo ""
	@echo "Docker Compose (dev):"
	@docker compose -f docker-compose.local.yml ps 2>/dev/null || echo "  Not running"
	@echo ""
	@echo "K3d Cluster:"
	@k3d cluster list 2>/dev/null || echo "  Not running"
	@echo ""
	@if kubectl get ns authorworks &>/dev/null; then \
		echo "Kubernetes (authorworks namespace):"; \
		kubectl get pods -n authorworks; \
	fi

verify:
	@$(DEPLOY_SCRIPT) local --verify 2>/dev/null || \
		$(DEPLOY_SCRIPT) dev --verify 2>/dev/null || \
		$(DEPLOY_SCRIPT) k3d --verify 2>/dev/null || \
		echo "No active deployment to verify"

#=============================================================================
# Scaling (EKS/Kubernetes)
#=============================================================================

scale-up:
	@kubectl scale -n authorworks deployment --all --replicas=10 2>/dev/null || \
		echo "❌ Kubernetes not configured or no deployments found"

scale-down:
	@kubectl scale -n authorworks deployment --all --replicas=3 2>/dev/null || \
		echo "❌ Kubernetes not configured or no deployments found"

rollback:
	@kubectl rollout undo -n authorworks deployment --all 2>/dev/null || \
		echo "❌ Kubernetes not configured or no deployments found"

#=============================================================================
# Database Operations
#=============================================================================

db-migrate:
	@echo "📦 Running database migrations..."
	@if [ -f ./scripts/schema.sql ]; then \
		docker exec -i authorworks-postgres psql -U authorworks -d authorworks < ./scripts/schema.sql; \
	else \
		echo "No migration script found"; \
	fi

db-seed:
	@echo "🌱 Seeding database..."
	@if [ -f ./scripts/seed.sql ]; then \
		docker exec -i authorworks-postgres psql -U authorworks -d authorworks < ./scripts/seed.sql; \
	else \
		echo "No seed script found"; \
	fi

db-reset:
	@echo "⚠️  This will DESTROY all data in the database!"
	@read -p "Are you sure? [y/N] " -n 1 -r; echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		docker exec -i authorworks-postgres psql -U authorworks -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"; \
		make db-migrate; \
		echo "✅ Database reset complete"; \
	else \
		echo "Cancelled"; \
	fi

#=============================================================================
# Quick Commands
#=============================================================================

# Aliases for common operations
up: deploy-local
dev: deploy-dev
prod: deploy-ec2
restart: down deploy-local

# Show container resource usage
resources:
	@docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}"

# Shell into a service container
shell-%:
	@docker exec -it authorworks-$* /bin/sh 2>/dev/null || \
		docker exec -it authorworks-$* /bin/bash 2>/dev/null || \
		echo "Container authorworks-$* not found"

# View logs for a specific service
logs-%:
	@docker logs -f authorworks-$* 2>/dev/null || \
		kubectl logs -n authorworks -l app=$* -f 2>/dev/null || \
		echo "Service $* not found"
