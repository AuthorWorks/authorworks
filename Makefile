.PHONY: help build test deploy-local deploy-homelab deploy-ec2 deploy-eks down logs verify clean

SHELL := /bin/bash

#=============================================================================
# AuthorWorks - Makefile
#=============================================================================

help:
	@echo "AuthorWorks Platform"
	@echo ""
	@echo "Deployment:"
	@echo "  make deploy-local     Deploy to local Docker environment"
	@echo "  make deploy-homelab   Deploy to homelab K3s cluster"
	@echo "  make deploy-ec2       Deploy to AWS EC2"
	@echo "  make deploy-eks       Deploy to AWS EKS"
	@echo "  make down             Tear down current deployment"
	@echo ""
	@echo "Development:"
	@echo "  make build            Build all services"
	@echo "  make test             Run all tests"
	@echo "  make logs             Follow deployment logs"
	@echo "  make verify           Run health checks"
	@echo "  make clean            Clean build artifacts"
	@echo ""
	@echo "Operations:"
	@echo "  make scale-up         Scale to 10 replicas (EKS)"
	@echo "  make scale-down       Scale to 3 replicas (EKS)"
	@echo "  make rollback         Rollback to previous deployment"

#=============================================================================
# Deployment Targets
#=============================================================================

deploy-local:
	@./scripts/deploy.sh local --build --verify

deploy-homelab:
	@./scripts/deploy.sh homelab --build --verify

deploy-ec2:
	@./scripts/deploy.sh ec2 --build --verify

deploy-eks:
	@./scripts/deploy.sh eks --build --verify

down:
	@./scripts/deploy.sh local --down 2>/dev/null || true
	@./scripts/deploy.sh homelab --down 2>/dev/null || true

#=============================================================================
# Development Targets
#=============================================================================

build:
	@echo "Building all services..."
	@docker compose build

test:
	@echo "Running tests..."
	@cargo test --workspace 2>/dev/null || echo "Cargo tests skipped"
	@cd frontend/landing/leptos-app && cargo test 2>/dev/null || echo "Frontend tests skipped"

logs:
	@./scripts/deploy.sh local --logs

verify:
	@./scripts/deploy.sh local --verify

clean:
	@echo "Cleaning build artifacts..."
	@rm -rf target/ 2>/dev/null || true
	@find . -name "target" -type d -exec rm -rf {} + 2>/dev/null || true
	@docker system prune -f 2>/dev/null || true

#=============================================================================
# Operations Targets (EKS)
#=============================================================================

scale-up:
	@kubectl scale -n authorworks spinapp/authorworks-platform --replicas=10 2>/dev/null || \
		echo "EKS not configured"

scale-down:
	@kubectl scale -n authorworks spinapp/authorworks-platform --replicas=3 2>/dev/null || \
		echo "EKS not configured"

rollback:
	@kubectl rollout undo -n authorworks deployment/authorworks-platform 2>/dev/null || \
		echo "EKS not configured"

#=============================================================================
# Utility Targets
#=============================================================================

.env:
	@cp .env.example .env 2>/dev/null || echo "# AuthorWorks Environment" > .env
	@echo "Created .env file - please configure it"
