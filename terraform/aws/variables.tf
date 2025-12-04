# AuthorWorks AWS Infrastructure - Variables

#=============================================================================
# General
#=============================================================================

variable "aws_region" {
  description = "AWS region for resources"
  type        = string
  default     = "us-west-2"
}

variable "environment" {
  description = "Environment name (development, staging, production)"
  type        = string
  default     = "production"
  
  validation {
    condition     = contains(["development", "staging", "production"], var.environment)
    error_message = "Environment must be one of: development, staging, production"
  }
}

variable "project_name" {
  description = "Project name for resource naming"
  type        = string
  default     = "authorworks"
}

#=============================================================================
# Networking
#=============================================================================

variable "vpc_cidr" {
  description = "CIDR block for VPC"
  type        = string
  default     = "10.0.0.0/16"
}

variable "private_subnet_cidrs" {
  description = "CIDR blocks for private subnets"
  type        = list(string)
  default     = ["10.0.1.0/24", "10.0.2.0/24", "10.0.3.0/24"]
}

variable "public_subnet_cidrs" {
  description = "CIDR blocks for public subnets"
  type        = list(string)
  default     = ["10.0.101.0/24", "10.0.102.0/24", "10.0.103.0/24"]
}

#=============================================================================
# EKS
#=============================================================================

variable "cluster_name" {
  description = "Name of the EKS cluster"
  type        = string
  default     = "authorworks-eks"
}

variable "cluster_version" {
  description = "Kubernetes version for EKS cluster"
  type        = string
  default     = "1.28"
}

variable "node_instance_types" {
  description = "Instance types for EKS node group"
  type        = list(string)
  default     = ["t3.large", "t3.xlarge"]
}

variable "node_min_size" {
  description = "Minimum number of nodes in EKS node group"
  type        = number
  default     = 2
}

variable "node_max_size" {
  description = "Maximum number of nodes in EKS node group"
  type        = number
  default     = 10
}

variable "node_desired_size" {
  description = "Desired number of nodes in EKS node group"
  type        = number
  default     = 3
}

#=============================================================================
# RDS
#=============================================================================

variable "db_instance_class" {
  description = "RDS instance class"
  type        = string
  default     = "db.t3.medium"
}

variable "db_password" {
  description = "Password for RDS database"
  type        = string
  sensitive   = true
}

#=============================================================================
# ElastiCache
#=============================================================================

variable "redis_node_type" {
  description = "ElastiCache node type"
  type        = string
  default     = "cache.t3.medium"
}

variable "redis_auth_token" {
  description = "Auth token for Redis"
  type        = string
  sensitive   = true
}

#=============================================================================
# Application Secrets
#=============================================================================

variable "jwt_secret" {
  description = "Secret key for JWT tokens"
  type        = string
  sensitive   = true
}

variable "logto_client_id" {
  description = "Logto OAuth client ID"
  type        = string
}

variable "logto_client_secret" {
  description = "Logto OAuth client secret"
  type        = string
  sensitive   = true
}

variable "stripe_secret_key" {
  description = "Stripe secret API key"
  type        = string
  sensitive   = true
  default     = ""
}

variable "anthropic_api_key" {
  description = "Anthropic API key for content generation"
  type        = string
  sensitive   = true
  default     = ""
}

#=============================================================================
# Domain
#=============================================================================

variable "domain" {
  description = "Domain name for the application"
  type        = string
  default     = "authorworks.io"
}

variable "certificate_arn" {
  description = "ARN of ACM certificate for the domain"
  type        = string
  default     = ""
}
