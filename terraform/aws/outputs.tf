# AuthorWorks AWS Infrastructure - Outputs

#=============================================================================
# EKS
#=============================================================================

output "cluster_name" {
  description = "EKS cluster name"
  value       = module.eks.cluster_name
}

output "cluster_endpoint" {
  description = "EKS cluster endpoint"
  value       = module.eks.cluster_endpoint
}

output "cluster_certificate_authority_data" {
  description = "Base64 encoded certificate data for cluster authentication"
  value       = module.eks.cluster_certificate_authority_data
  sensitive   = true
}

output "cluster_security_group_id" {
  description = "Security group ID for cluster"
  value       = module.eks.cluster_security_group_id
}

output "oidc_provider_arn" {
  description = "ARN of OIDC provider for IRSA"
  value       = module.eks.oidc_provider_arn
}

#=============================================================================
# RDS
#=============================================================================

output "database_endpoint" {
  description = "RDS instance endpoint"
  value       = module.rds.db_instance_endpoint
}

output "database_name" {
  description = "RDS database name"
  value       = module.rds.db_instance_name
}

output "database_username" {
  description = "RDS master username"
  value       = module.rds.db_instance_username
  sensitive   = true
}

#=============================================================================
# ElastiCache
#=============================================================================

output "redis_endpoint" {
  description = "ElastiCache Redis endpoint"
  value       = aws_elasticache_replication_group.redis.primary_endpoint_address
}

output "redis_port" {
  description = "ElastiCache Redis port"
  value       = aws_elasticache_replication_group.redis.port
}

#=============================================================================
# S3
#=============================================================================

output "content_bucket_name" {
  description = "S3 bucket name for content storage"
  value       = aws_s3_bucket.content.id
}

output "content_bucket_arn" {
  description = "S3 bucket ARN for content storage"
  value       = aws_s3_bucket.content.arn
}

#=============================================================================
# ECR
#=============================================================================

output "ecr_repository_url" {
  description = "ECR repository URL"
  value       = aws_ecr_repository.authorworks.repository_url
}

#=============================================================================
# Secrets Manager
#=============================================================================

output "secrets_arn" {
  description = "ARN of Secrets Manager secret"
  value       = aws_secretsmanager_secret.authorworks.arn
}

#=============================================================================
# IAM
#=============================================================================

output "authorworks_irsa_role_arn" {
  description = "IAM role ARN for AuthorWorks service account"
  value       = module.authorworks_irsa.iam_role_arn
}

#=============================================================================
# VPC
#=============================================================================

output "vpc_id" {
  description = "VPC ID"
  value       = module.vpc.vpc_id
}

output "private_subnets" {
  description = "Private subnet IDs"
  value       = module.vpc.private_subnets
}

output "public_subnets" {
  description = "Public subnet IDs"
  value       = module.vpc.public_subnets
}

#=============================================================================
# Connection Strings (for reference)
#=============================================================================

output "kubectl_config_command" {
  description = "Command to configure kubectl"
  value       = "aws eks update-kubeconfig --region ${var.aws_region} --name ${module.eks.cluster_name}"
}

output "deployment_instructions" {
  description = "Instructions for deploying to the cluster"
  value       = <<-EOT
    # Configure kubectl
    aws eks update-kubeconfig --region ${var.aws_region} --name ${module.eks.cluster_name}
    
    # Verify cluster access
    kubectl get nodes
    
    # Deploy AuthorWorks
    cd /path/to/authorworks
    make deploy-aws
    
    # Check deployment status
    kubectl get pods -n authorworks
    kubectl get spinapp -n authorworks
  EOT
}

