.PHONY: help all build test deploy-homelab deploy-aws clean verify health-check

SHELL := /bin/bash
REGISTRY ?= ghcr.io/authorworks
IMAGE_TAG ?= $(shell git rev-parse --short HEAD)
NAMESPACE ?= authorworks
PROFILE ?= release

help:
	@echo "AuthorWorks Platform - Deployment & Management"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Main Targets:"
	@echo "  all                Build everything (SPIN + containers)"
	@echo "  verify             Pre-deployment verification checks"
	@echo "  deploy-homelab     Deploy to K3S homelab cluster"
	@echo "  deploy-aws         Deploy to AWS EKS production"
	@echo "  health-check       Run health checks on deployed services"
	@echo ""
	@echo "Build Targets:"
	@echo "  build-spin         Build SPIN WebAssembly application"
	@echo "  build-containers   Build containerized services"
	@echo "  build-optimized    Build with WASM optimizations"
	@echo ""
	@echo "Testing:"
	@echo "  test               Run all tests"
	@echo "  test-integration   Run integration tests"
	@echo "  benchmark          Run performance benchmarks"
	@echo ""
	@echo "Maintenance:"
	@echo "  clean              Clean all build artifacts"
	@echo "  logs               Tail application logs"
	@echo "  rollback           Rollback to previous deployment"

all: verify build-spin build-containers

verify:
	@echo "🔍 Running pre-deployment verification..."
	@./scripts/verify-deployment.sh

build-spin:
	@echo "🔨 Building SPIN WebAssembly application..."
	@PROFILE=$(PROFILE) ./scripts/build-spin.sh

build-optimized:
	@echo "⚡ Building optimized SPIN application..."
	@PROFILE=$(PROFILE) OPTIMIZE=true ./scripts/build-spin.sh

build-containers:
	@echo "🐳 Building container images..."
	@docker build -f Dockerfile.spin -t $(REGISTRY)/authorworks-platform:$(IMAGE_TAG) .
	@docker tag $(REGISTRY)/authorworks-platform:$(IMAGE_TAG) $(REGISTRY)/authorworks-platform:latest

push:
	@echo "📤 Pushing images to registry..."
	@docker push $(REGISTRY)/authorworks-platform:$(IMAGE_TAG)
	@docker push $(REGISTRY)/authorworks-platform:latest

test:
	@echo "🧪 Running all tests..."
	@cargo test --workspace

test-integration:
	@echo "🔗 Running integration tests..."
	@./scripts/run-integration-tests.sh

benchmark:
	@echo "📊 Running performance benchmarks..."
	@./scripts/benchmark-wasm.sh

deploy-homelab: verify build-optimized
	@echo "🚀 Deploying to K3S homelab cluster..."
	@CLUSTER_CONTEXT=k3s-homelab ./scripts/deploy-homelab.sh

deploy-aws: verify build-optimized push
	@echo "☁️  Deploying to AWS EKS..."
	@CLUSTER_CONTEXT=aws-eks ./scripts/deploy-aws.sh

health-check:
	@echo "❤️  Running health checks..."
	@kubectl exec -n $(NAMESPACE) deployment/authorworks-platform -- /app/scripts/health-check.sh

logs:
	@echo "📋 Tailing application logs..."
	@kubectl logs -n $(NAMESPACE) -l app=authorworks --tail=100 -f

rollback:
	@echo "↩️  Rolling back deployment..."
	@kubectl rollout undo -n $(NAMESPACE) deployment/authorworks-platform

clean:
	@echo "🧹 Cleaning build artifacts..."
	@rm -rf target/
	@find . -name "target" -type d -exec rm -rf {} + 2>/dev/null || true
	@docker system prune -f

monitor:
	@echo "📊 Opening monitoring dashboard..."
	@kubectl port-forward -n $(NAMESPACE) svc/grafana 3000:3000 &
	@open http://localhost:3000

scale-up:
	@echo "⬆️  Scaling up application..."
	@kubectl scale -n $(NAMESPACE) spinapp/authorworks-platform --replicas=10

scale-down:
	@echo "⬇️  Scaling down application..."
	@kubectl scale -n $(NAMESPACE) spinapp/authorworks-platform --replicas=3