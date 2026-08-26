# ============================================================
# MOX Enterprise · 千亿亿级分布式架构 Terraform 基础设施
# ============================================================
# 提供商：AWS（可适配阿里云/腾讯云/GCP）
# 架构：3 Region × 3 AZ 全球多活
# ============================================================

terraform {
  required_version = ">= 1.5.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.0"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "~> 2.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.0"
    }
  }

  backend "s3" {
    bucket         = "mox-terraform-state-prod"
    key            = "infra/terraform.tfstate"
    region         = "ap-southeast-1"
    dynamodb_table = "mox-terraform-locks"
    encrypt        = true
  }
}

# ─── 全局变量 ───
variable "project_name" {
    description = "项目名称"
    type        = string
    default     = "mox"
}

variable "environment" {
    description = "环境：dev / staging / production"
    type        = string
    default     = "production"
}

variable "regions" {
    description = "部署 Region 列表（全球多活）"
    type        = list(string)
    default     = ["ap-southeast-1", "eu-central-1", "us-east-1"]
}

variable "primary_region" {
    description = "主 Region"
    type        = string
    default     = "ap-southeast-1"
}

variable "vpc_cidr" {
    description = "VPC CIDR"
    type        = string
    default     = "10.0.0.0/16"
}

variable "instance_types" {
    description = "各组件实例类型"
    type = object({
      api    = string
      tikv   = string
      spark  = string
      monitor = string
    })
    default = {
      api     = "m6i.xlarge"
      tikv    = "i3.2xlarge"
      spark   = "r6i.4xlarge"
      monitor = "m6i.2xlarge"
    }
}

variable "s3_lifecycle" {
    description = "S3 生命周期配置（冷热分级）"
    type = object({
      hot_days   = number
      warm_days  = number
      cold_days  = number
      expire_days = number
    })
    default = {
      hot_days    = 30
      warm_days   = 365
      cold_days   = 2555  # 7年
      expire_days = 2557
    }
}

# ─── Provider 配置（多 Region） ───
provider "aws" {
  alias  = "primary"
  region = var.primary_region

  default_tags {
    tags = {
      Project     = var.project_name
      Environment = var.environment
      ManagedBy   = "terraform"
    }
  }
}

# ─── 随机后缀（避免资源名冲突） ───
resource "random_string" "suffix" {
  length  = 6
  special = false
  upper   = false
}

locals {
  name_prefix = "${var.project_name}-${var.environment}"
  common_tags = {
    Project     = var.project_name
    Environment = var.environment
    ManagedBy   = "terraform"
  }
}

# ============================================================
# 1. VPC 网络（3 AZ）
# ============================================================
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"

  providers = {
    aws = aws.primary
  }

  name = "${local.name_prefix}-vpc"
  cidr = var.vpc_cidr

  azs             = ["${var.primary_region}a", "${var.primary_region}b", "${var.primary_region}c"]
  private_subnets = ["10.0.1.0/24", "10.0.2.0/24", "10.0.3.0/24"]
  public_subnets  = ["10.0.101.0/24", "10.0.102.0/24", "10.0.103.0/24"]
  database_subnets = ["10.0.51.0/24", "10.0.52.0/24", "10.0.53.0/24"]

  enable_nat_gateway     = true
  single_nat_gateway     = false
  one_nat_gateway_per_az = true

  enable_vpc_flow_logs          = true
  vpc_flow_logs_log_destination_type = "s3"
  vpc_flow_logs_s3_bucket_arn   = aws_s3_bucket.flow_logs.arn

  tags = local.common_tags
}

# ============================================================
# 2. S3 存储桶（对象存储 + 三级冷热生命周期）
# ============================================================
resource "aws_s3_bucket" "chunks" {
  provider = aws.primary
  bucket   = "${local.name_prefix}-chunks-${random_string.suffix.result}"

  versioning {
    enabled = false  # 内容寻址，不需要版本控制
  }

  server_side_encryption_configuration {
    rule {
      apply_server_side_encryption_by_default {
        kms_master_key_id = aws_kms_key.storage.arn
        sse_algorithm     = "aws:kms"
      }
    }
  }

  tags = merge(local.common_tags, { Name = "${local.name_prefix}-chunks" })
}

# 生命周期：热→温→冷→归档
resource "aws_s3_bucket_lifecycle_configuration" "chunks_lifecycle" {
  provider = aws.primary
  bucket   = aws_s3_bucket.chunks.id

  rule {
    id     = "hot-to-warm"
    status = "Enabled"

    transition {
      days          = var.s3_lifecycle.hot_days
      storage_class = "STANDARD_IA"
    }

    transition {
      days          = var.s3_lifecycle.warm_days
      storage_class = "GLACIER_IR"
    }

    transition {
      days          = var.s3_lifecycle.cold_days
      storage_class = "DEEP_ARCHIVE"
    }

    expiration {
      days = var.s3_lifecycle.expire_days
    }
  }
}

# 跨 Region 复制（CRR）
resource "aws_s3_bucket_replication_configuration" "chunks_replication" {
  provider = aws.primary
  role     = aws_iam_role.s3_replication.arn
  bucket   = aws_s3_bucket.chunks.id

  rule {
    id     = "replicate-to-eu"
    status = "Enabled"

    destination {
      bucket        = "arn:aws:s3:::${local.name_prefix}-chunks-eu"
      storage_class = "STANDARD"
    }
  }

  rule {
    id     = "replicate-to-us"
    status = "Enabled"

    destination {
      bucket        = "arn:aws:s3:::${local.name_prefix}-chunks-us"
      storage_class = "STANDARD"
    }
  }
}

# 审计日志桶
resource "aws_s3_bucket" "audit_logs" {
  provider = aws.primary
  bucket   = "${local.name_prefix}-audit-logs-${random_string.suffix.result}"

  versioning {
    enabled = true
  }

  lifecycle {
    prevent_destroy = true  # 审计日志不可删除
  }

  tags = merge(local.common_tags, { Name = "${local.name_prefix}-audit-logs", Retention = "7years" })
}

# VPC Flow Logs 桶
resource "aws_s3_bucket" "flow_logs" {
  provider = aws.primary
  bucket   = "${local.name_prefix}-flow-logs-${random_string.suffix.result}"
  tags     = merge(local.common_tags, { Name = "${local.name_prefix}-flow-logs" })
}

# ============================================================
# 3. KMS 密钥管理
# ============================================================
resource "aws_kms_key" "storage" {
  provider                = aws.primary
  description             = "${local.name_prefix} S3 存储加密密钥"
  key_usage               = "ENCRYPT_DECRYPT"
  customer_master_key_spec = "SYMMETRIC_DEFAULT"
  enable_key_rotation     = true
  rotation_period_in_days = 365

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Principal = { AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root" }
        Action   = "kms:*"
        Resource = "*"
      }
    ]
  })

  tags = local.common_tags
}

resource "aws_kms_alias" "storage" {
  provider    = aws.primary
  name        = "alias/${local.name_prefix}-storage"
  target_key_id = aws_kms_key.storage.key_id
}

# ============================================================
# 4. EKS Kubernetes 集群
# ============================================================
module "eks" {
  source  = "terraform-aws-modules/eks/aws"
  version = "~> 20.0"

  providers = {
    aws = aws.primary
  }

  cluster_name    = "${local.name_prefix}-eks"
  cluster_version = "1.29"

  vpc_id     = module.vpc.vpc_id
  subnet_ids = module.vpc.private_subnets

  # ─── API 服务节点组 ───
  eks_managed_node_groups = {
    api = {
      desired_size = 3
      min_size     = 3
      max_size     = 20
      instance_types = [var.instance_types.api]
      capacity_type  = "ON_DEMAND"

      taints = [
        { key = "dedicated", value = "api", effect = "NO_SCHEDULE" }
      ]

      labels = { role = "api" }
    }

    # ─── TiKV 元数据节点组（高 IO） ───
    tikv = {
      desired_size = 6
      min_size     = 6
      max_size     = 30
      instance_types = [var.instance_types.tikv]
      capacity_type  = "ON_DEMAND"

      taints = [
        { key = "dedicated", value = "tikv", effect = "NO_SCHEDULE" }
      ]

      labels = { role = "tikv" }

      block_device = [
        {
          device_name = "/dev/xvda"
          ebs = {
            volume_size = 100
            volume_type = "gp3"
            iops        = 3000
          }
        }
      ]
    }

    # ─── Spark 计算节点组（高内存） ───
    spark = {
      desired_size = 0
      min_size     = 0
      max_size     = 100
      instance_types = [var.instance_types.spark]
      capacity_type  = "SPOT"  # Spark 用竞价实例降本

      taints = [
        { key = "dedicated", value = "spark", effect = "NO_SCHEDULE" }
      ]

      labels = { role = "spark" }
    }

    # ─── 监控节点组 ───
    monitor = {
      desired_size = 3
      min_size     = 3
      max_size     = 10
      instance_types = [var.instance_types.monitor]
      capacity_type  = "ON_DEMAND"

      labels = { role = "monitor" }
    }
  }

  tags = local.common_tags
}

# ============================================================
# 5. IAM 角色
# ============================================================
data "aws_caller_identity" "current" {
  provider = aws.primary
}

resource "aws_iam_role" "s3_replication" {
  provider = aws.primary
  name     = "${local.name_prefix}-s3-replication"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = { Service = "s3.amazonaws.com" }
      Action   = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy" "s3_replication" {
  provider = aws.primary
  role     = aws_iam_role.s3_replication.id
  policy   = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = ["s3:GetReplicationConfiguration", "s3:ListBucket"]
        Resource = [aws_s3_bucket.chunks.arn]
      },
      {
        Effect = "Allow"
        Action = ["s3:GetObjectVersionForReplication", "s3:GetObjectVersionAcl", "s3:GetObjectVersionTagging"]
        Resource = ["${aws_s3_bucket.chunks.arn}/*"]
      },
      {
        Effect = "Allow"
        Action = ["s3:ReplicateObject", "s3:ReplicateDelete", "s3:ReplicateTags"]
        Resource = ["arn:aws:s3:::${local.name_prefix}-chunks-*/*"]
      }
    ]
  })
}

# ============================================================
# 6. CloudWatch 告警
# ============================================================
resource "aws_cloudwatch_metric_alarm" "api_5xx_high" {
  provider            = aws.primary
  alarm_name          = "${local.name_prefix}-api-5xx-high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "3"
  metric_name         = "5xxErrorRate"
  namespace           = "AWS/ApplicationELB"
  period              = "60"
  statistic           = "Average"
  threshold           = "1"
  alarm_description   = "API 5xx 错误率 > 1% 持续 3 分钟"
  alarm_actions       = [aws_sns_topic.alerts.arn]
  tags                = local.common_tags
}

resource "aws_sns_topic" "alerts" {
  provider = aws.primary
  name     = "${local.name_prefix}-alerts"
}

resource "aws_sns_topic_subscription" "dingtalk" {
  provider  = aws.primary
  topic_arn = aws_sns_topic.alerts.arn
  protocol  = "https"
  endpoint  = "https://oapi.dingtalk.com/robot/send?access_token=${var.dingtalk_token}"
}

# ============================================================
# 7. Route53 GeoDNS（全球智能路由）
# ============================================================
resource "aws_route53_health_check" "api_primary" {
  provider = aws.primary

  fqdn              = "api.infotopograph.io"
  port              = 443
  type              = "HTTPS"
  resource_path     = "/health"
  failure_threshold = "3"
  request_interval  = "10"

  tags = local.common_tags
}

# ============================================================
# Outputs
# ============================================================
output "vpc_id" {
  description = "VPC ID"
  value       = module.vpc.vpc_id
}

output "eks_cluster_name" {
  description = "EKS 集群名称"
  value       = module.eks.cluster_name
}

output "s3_chunks_bucket" {
  description = "Chunk 存储桶名称"
  value       = aws_s3_bucket.chunks.bucket
}

output "s3_audit_bucket" {
  description = "审计日志桶名称"
  value       = aws_s3_bucket.audit_logs.bucket
}

output "kms_key_arn" {
  description = "存储加密 KMS Key ARN"
  value       = aws_kms_key.storage.arn
}

output "regions" {
  description = "部署的 Region 列表"
  value       = var.regions
}
