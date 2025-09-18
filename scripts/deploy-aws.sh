#!/bin/bash
set -e

echo "🚀 Deploying AuthorWorks to AWS EKS with SpinKube"

# Check prerequisites
command -v terraform >/dev/null 2>&1 || { echo "terraform is required but not installed. Aborting." >&2; exit 1; }
command -v aws >/dev/null 2>&1 || { echo "AWS CLI is required but not installed. Aborting." >&2; exit 1; }
command -v kubectl >/dev/null 2>&1 || { echo "kubectl is required but not installed. Aborting." >&2; exit 1; }
command -v spin >/dev/null 2>&1 || { echo "spin CLI is required but not installed. Aborting." >&2; exit 1; }

# Set variables
AWS_REGION=${AWS_REGION:-"us-west-2"}
ENVIRONMENT=${ENVIRONMENT:-"production"}
PROJECT_NAME="authorworks"

echo "📦 Setting up AWS infrastructure with Terraform..."
cd terraform/aws

# Initialize Terraform
terraform init

# Plan deployment
terraform plan -var="environment=${ENVIRONMENT}" -out=tfplan

# Apply infrastructure
terraform apply tfplan

# Get EKS cluster details
CLUSTER_NAME=$(terraform output -raw cluster_name)
ECR_REPOSITORY=$(terraform output -raw ecr_repository_url)

echo "🔑 Configuring kubectl for EKS..."
aws eks update-kubeconfig --region ${AWS_REGION} --name ${CLUSTER_NAME}

echo "🔨 Building Spin application..."
cd ../..
spin build

echo "📤 Building and pushing Docker image with Spin app..."
# Build container image with Spin app
docker build -f Dockerfile.spin -t ${ECR_REPOSITORY}:latest .

# Login to ECR
aws ecr get-login-password --region ${AWS_REGION} | docker login --username AWS --password-stdin ${ECR_REPOSITORY}

# Push image
docker push ${ECR_REPOSITORY}:latest

echo "🚀 Deploying with Helm..."
helm upgrade --install authorworks charts/authorworks \
  --namespace authorworks \
  --create-namespace \
  --set image.repository=${ECR_REPOSITORY} \
  --set image.tag=latest \
  --set environment=production

echo "⏳ Waiting for deployment to be ready..."
kubectl wait --for=condition=ready --timeout=600s \
  spinapp/authorworks-platform \
  -n authorworks

echo "✅ Deployment complete!"
echo ""
echo "📊 Application status:"
kubectl get spinapp -n authorworks
echo ""
echo "🌐 Getting Load Balancer URL..."
kubectl get ingress -n authorworks