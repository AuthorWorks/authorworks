# Install SpinKube operator on EKS
resource "helm_release" "spin_operator" {
  name       = "spin-operator"
  repository = "https://spinkube.github.io/charts"
  chart      = "spin-operator"
  namespace  = "spin-system"
  version    = "0.2.0"

  create_namespace = true

  values = [
    yamlencode({
      operator = {
        image = {
          repository = "ghcr.io/spinkube/spin-operator"
          tag        = "v0.2.0"
        }
        resources = {
          limits = {
            cpu    = "200m"
            memory = "256Mi"
          }
          requests = {
            cpu    = "100m"
            memory = "128Mi"
          }
        }
      }
      shim = {
        executor = "containerd-shim-spin"
      }
      runtimeClass = {
        name = "spin"
      }
    })
  ]

  depends_on = [module.eks]
}

# Install containerd-shim-spin runtime class
resource "helm_release" "containerd_shim_spin" {
  name       = "containerd-shim-spin"
  repository = "https://spinkube.github.io/charts"
  chart      = "containerd-shim-spin-installer"
  namespace  = "spin-system"
  version    = "0.14.1"

  values = [
    yamlencode({
      installer = {
        image = {
          repository = "ghcr.io/spinkube/containerd-shim-spin-installer"
          tag        = "v0.14.1"
        }
      }
    })
  ]

  depends_on = [
    module.eks,
    helm_release.spin_operator
  ]
}

# Deploy AuthorWorks Spin application
resource "helm_release" "authorworks" {
  name       = "authorworks"
  chart      = "../../charts/authorworks"
  namespace  = "authorworks"

  create_namespace = true

  values = [
    yamlencode({
      image = {
        repository = "${aws_ecr_repository.authorworks.repository_url}"
        tag        = "latest"
      }
      
      replicaCount = 5
      
      autoscaling = {
        enabled                        = true
        minReplicas                   = 5
        maxReplicas                   = 100
        targetCPUUtilizationPercentage = 60
      }
      
      resources = {
        limits = {
          cpu    = "2000m"
          memory = "4Gi"
        }
        requests = {
          cpu    = "500m"
          memory = "1Gi"
        }
      }
      
      postgresql = {
        enabled = false
        external = {
          host     = module.rds.db_instance_endpoint
          port     = 5432
          database = "authorworks"
          username = "authorworks"
        }
      }
      
      redis = {
        enabled = false
        external = {
          host = aws_elasticache_cluster.redis.cache_nodes[0].address
          port = 6379
        }
      }
      
      minio = {
        enabled = false
        external = {
          endpoint = "s3.amazonaws.com"
          bucket   = aws_s3_bucket.authorworks_storage.id
          region   = var.aws_region
        }
      }
      
      ingress = {
        enabled   = true
        className = "alb"
        annotations = {
          "alb.ingress.kubernetes.io/scheme"      = "internet-facing"
          "alb.ingress.kubernetes.io/target-type" = "ip"
        }
        hosts = [
          {
            host = "api.authorworks.io"
            paths = [
              {
                path     = "/"
                pathType = "Prefix"
              }
            ]
          }
        ]
      }
      
      multiTenancy = {
        enabled = true
        tenants = [
          {
            name      = "enterprise-1"
            namespace = "authorworks-enterprise-1"
            replicas  = 3
            resources = {
              limits = {
                cpu    = "2000m"
                memory = "4Gi"
              }
              requests = {
                cpu    = "500m"
                memory = "1Gi"
              }
            }
          },
          {
            name      = "enterprise-2"
            namespace = "authorworks-enterprise-2"
            replicas  = 3
            resources = {
              limits = {
                cpu    = "2000m"
                memory = "4Gi"
              }
              requests = {
                cpu    = "500m"
                memory = "1Gi"
              }
            }
          }
        ]
      }
    })
  ]

  depends_on = [
    helm_release.spin_operator,
    helm_release.containerd_shim_spin,
    module.rds,
    aws_elasticache_cluster.redis,
    aws_s3_bucket.authorworks_storage
  ]
}

# ECR Repository for Spin images
resource "aws_ecr_repository" "authorworks" {
  name                 = "${var.project_name}-spin"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }

  encryption_configuration {
    encryption_type = "AES256"
  }

  tags = local.tags
}

# ECR Lifecycle Policy
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
      }
    ]
  })
}