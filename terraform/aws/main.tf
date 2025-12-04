# AuthorWorks AWS Infrastructure
# Terraform configuration for EKS-based production deployment

terraform {
  required_version = ">= 1.5.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.23"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "~> 2.11"
    }
  }

  backend "s3" {
    bucket         = "authorworks-terraform-state"
    key            = "aws/terraform.tfstate"
    region         = "us-west-2"
    encrypt        = true
    dynamodb_table = "authorworks-terraform-locks"
  }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "AuthorWorks"
      Environment = var.environment
      ManagedBy   = "Terraform"
    }
  }
}

provider "kubernetes" {
  host                   = module.eks.cluster_endpoint
  cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)
  
  exec {
    api_version = "client.authentication.k8s.io/v1beta1"
    command     = "aws"
    args        = ["eks", "get-token", "--cluster-name", module.eks.cluster_name]
  }
}

provider "helm" {
  kubernetes {
    host                   = module.eks.cluster_endpoint
    cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)
    
    exec {
      api_version = "client.authentication.k8s.io/v1beta1"
      command     = "aws"
      args        = ["eks", "get-token", "--cluster-name", module.eks.cluster_name]
    }
  }
}

#=============================================================================
# Data Sources
#=============================================================================

data "aws_availability_zones" "available" {
  state = "available"
}

data "aws_caller_identity" "current" {}

#=============================================================================
# VPC
#=============================================================================

module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"

  name = "${var.project_name}-vpc"
  cidr = var.vpc_cidr

  azs             = slice(data.aws_availability_zones.available.names, 0, 3)
  private_subnets = var.private_subnet_cidrs
  public_subnets  = var.public_subnet_cidrs

  enable_nat_gateway     = true
  single_nat_gateway     = var.environment != "production"
  enable_dns_hostnames   = true
  enable_dns_support     = true

  # Tags required for EKS
  public_subnet_tags = {
    "kubernetes.io/role/elb"                    = 1
    "kubernetes.io/cluster/${var.cluster_name}" = "owned"
  }

  private_subnet_tags = {
    "kubernetes.io/role/internal-elb"           = 1
    "kubernetes.io/cluster/${var.cluster_name}" = "owned"
  }
}

#=============================================================================
# EKS Cluster
#=============================================================================

module "eks" {
  source  = "terraform-aws-modules/eks/aws"
  version = "~> 19.0"

  cluster_name    = var.cluster_name
  cluster_version = var.cluster_version

  vpc_id     = module.vpc.vpc_id
  subnet_ids = module.vpc.private_subnets

  cluster_endpoint_public_access  = true
  cluster_endpoint_private_access = true

  # EKS Managed Node Groups
  eks_managed_node_groups = {
    general = {
      name           = "general-workloads"
      instance_types = var.node_instance_types
      
      min_size     = var.node_min_size
      max_size     = var.node_max_size
      desired_size = var.node_desired_size

      labels = {
        workload = "general"
      }

      tags = {
        ExtraTag = "AuthorWorks-General"
      }
    }

    spin = {
      name           = "spin-workloads"
      instance_types = ["t3.medium", "t3.large"]
      
      min_size     = 1
      max_size     = 5
      desired_size = 2

      labels = {
        workload = "spin"
        "spinkube.dev/node-type" = "spin"
      }

      taints = [{
        key    = "spin"
        value  = "true"
        effect = "NO_SCHEDULE"
      }]

      tags = {
        ExtraTag = "AuthorWorks-Spin"
      }
    }
  }

  # OIDC Provider
  enable_irsa = true

  # Cluster Add-ons
  cluster_addons = {
    coredns = {
      most_recent = true
    }
    kube-proxy = {
      most_recent = true
    }
    vpc-cni = {
      most_recent = true
    }
    aws-ebs-csi-driver = {
      most_recent              = true
      service_account_role_arn = module.ebs_csi_irsa_role.iam_role_arn
    }
  }

  # aws-auth configmap
  manage_aws_auth_configmap = true
}

# EBS CSI Driver IAM Role
module "ebs_csi_irsa_role" {
  source  = "terraform-aws-modules/iam/aws//modules/iam-role-for-service-accounts-eks"
  version = "~> 5.0"

  role_name             = "${var.cluster_name}-ebs-csi"
  attach_ebs_csi_policy = true

  oidc_providers = {
    main = {
      provider_arn               = module.eks.oidc_provider_arn
      namespace_service_accounts = ["kube-system:ebs-csi-controller-sa"]
    }
  }
}

#=============================================================================
# RDS PostgreSQL
#=============================================================================

module "rds" {
  source  = "terraform-aws-modules/rds/aws"
  version = "~> 6.0"

  identifier = "${var.project_name}-postgres"

  engine               = "postgres"
  engine_version       = "16.1"
  family               = "postgres16"
  major_engine_version = "16"
  instance_class       = var.db_instance_class

  allocated_storage     = 20
  max_allocated_storage = 100

  db_name  = "authorworks"
  username = "authorworks"
  port     = 5432

  # Multi-AZ for production
  multi_az = var.environment == "production"

  # Networking
  db_subnet_group_name   = module.vpc.database_subnet_group_name
  vpc_security_group_ids = [aws_security_group.rds.id]
  subnet_ids             = module.vpc.private_subnets

  # Backup
  backup_retention_period = var.environment == "production" ? 7 : 1
  skip_final_snapshot     = var.environment != "production"
  deletion_protection     = var.environment == "production"

  # Performance Insights
  performance_insights_enabled = true

  # Parameter group
  parameters = [
    {
      name  = "log_connections"
      value = "1"
    }
  ]

  create_db_subnet_group = true
}

resource "aws_security_group" "rds" {
  name_prefix = "${var.project_name}-rds-"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [module.eks.cluster_security_group_id]
  }
}

#=============================================================================
# ElastiCache Redis
#=============================================================================

resource "aws_elasticache_subnet_group" "redis" {
  name       = "${var.project_name}-redis"
  subnet_ids = module.vpc.private_subnets
}

resource "aws_security_group" "redis" {
  name_prefix = "${var.project_name}-redis-"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port       = 6379
    to_port         = 6379
    protocol        = "tcp"
    security_groups = [module.eks.cluster_security_group_id]
  }
}

resource "aws_elasticache_replication_group" "redis" {
  replication_group_id       = "${var.project_name}-redis"
  description                = "AuthorWorks Redis cluster"
  node_type                  = var.redis_node_type
  num_cache_clusters         = var.environment == "production" ? 2 : 1
  port                       = 6379
  
  subnet_group_name          = aws_elasticache_subnet_group.redis.name
  security_group_ids         = [aws_security_group.redis.id]
  
  automatic_failover_enabled = var.environment == "production"
  multi_az_enabled          = var.environment == "production"
  
  at_rest_encryption_enabled = true
  transit_encryption_enabled = true
  auth_token                 = var.redis_auth_token

  engine_version = "7.0"
  
  snapshot_retention_limit = var.environment == "production" ? 7 : 0
}

#=============================================================================
# S3 Buckets
#=============================================================================

resource "aws_s3_bucket" "content" {
  bucket = "${var.project_name}-content-${var.environment}"
}

resource "aws_s3_bucket_versioning" "content" {
  bucket = aws_s3_bucket.content.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "content" {
  bucket = aws_s3_bucket.content.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "content" {
  bucket = aws_s3_bucket.content.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

#=============================================================================
# ECR Repository
#=============================================================================

resource "aws_ecr_repository" "authorworks" {
  name                 = var.project_name
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }

  encryption_configuration {
    encryption_type = "AES256"
  }
}

resource "aws_ecr_lifecycle_policy" "authorworks" {
  repository = aws_ecr_repository.authorworks.name

  policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last 10 images"
        selection = {
          tagStatus     = "tagged"
          tagPrefixList = ["v"]
          countType     = "imageCountMoreThan"
          countNumber   = 10
        }
        action = {
          type = "expire"
        }
      },
      {
        rulePriority = 2
        description  = "Expire untagged images older than 7 days"
        selection = {
          tagStatus   = "untagged"
          countType   = "sinceImagePushed"
          countUnit   = "days"
          countNumber = 7
        }
        action = {
          type = "expire"
        }
      }
    ]
  })
}

#=============================================================================
# Secrets Manager
#=============================================================================

resource "aws_secretsmanager_secret" "authorworks" {
  name = "${var.project_name}/${var.environment}/secrets"
}

resource "aws_secretsmanager_secret_version" "authorworks" {
  secret_id = aws_secretsmanager_secret.authorworks.id
  secret_string = jsonencode({
    database_url       = "postgres://${module.rds.db_instance_username}:${var.db_password}@${module.rds.db_instance_endpoint}/${module.rds.db_instance_name}"
    redis_url          = "rediss://:${var.redis_auth_token}@${aws_elasticache_replication_group.redis.primary_endpoint_address}:6379"
    jwt_secret         = var.jwt_secret
    logto_client_id    = var.logto_client_id
    logto_client_secret = var.logto_client_secret
    stripe_secret_key  = var.stripe_secret_key
    anthropic_api_key  = var.anthropic_api_key
  })
}

#=============================================================================
# IAM Role for Service Accounts
#=============================================================================

module "authorworks_irsa" {
  source  = "terraform-aws-modules/iam/aws//modules/iam-role-for-service-accounts-eks"
  version = "~> 5.0"

  role_name = "${var.cluster_name}-authorworks"

  oidc_providers = {
    main = {
      provider_arn               = module.eks.oidc_provider_arn
      namespace_service_accounts = ["authorworks:authorworks"]
    }
  }

  role_policy_arns = {
    s3_access       = aws_iam_policy.s3_access.arn
    secrets_access  = aws_iam_policy.secrets_access.arn
  }
}

resource "aws_iam_policy" "s3_access" {
  name = "${var.project_name}-s3-access"
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:PutObject",
          "s3:DeleteObject",
          "s3:ListBucket"
        ]
        Resource = [
          aws_s3_bucket.content.arn,
          "${aws_s3_bucket.content.arn}/*"
        ]
      }
    ]
  })
}

resource "aws_iam_policy" "secrets_access" {
  name = "${var.project_name}-secrets-access"
  
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = [
          aws_secretsmanager_secret.authorworks.arn
        ]
      }
    ]
  })
}

#=============================================================================
# SpinKube Installation
#=============================================================================

resource "helm_release" "spin_operator" {
  name             = "spin-operator"
  repository       = "https://spinkube.github.io/charts"
  chart            = "spin-operator"
  namespace        = "spin-system"
  create_namespace = true
  version          = "0.2.0"

  depends_on = [module.eks]
}

resource "helm_release" "containerd_shim_spin" {
  name       = "containerd-shim-spin"
  repository = "https://spinkube.github.io/charts"
  chart      = "containerd-shim-spin-installer"
  namespace  = "spin-system"
  version    = "0.14.1"

  depends_on = [helm_release.spin_operator]
}

#=============================================================================
# Kubernetes Resources
#=============================================================================

resource "kubernetes_namespace" "authorworks" {
  metadata {
    name = "authorworks"
    labels = {
      "app.kubernetes.io/name"    = "authorworks"
      "app.kubernetes.io/part-of" = "authorworks-platform"
    }
  }

  depends_on = [module.eks]
}

resource "kubernetes_service_account" "authorworks" {
  metadata {
    name      = "authorworks"
    namespace = kubernetes_namespace.authorworks.metadata[0].name
    annotations = {
      "eks.amazonaws.com/role-arn" = module.authorworks_irsa.iam_role_arn
    }
  }
}
