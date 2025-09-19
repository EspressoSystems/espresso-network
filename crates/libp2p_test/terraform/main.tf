# S3 bucket is in us-east-2
provider "aws" {
  alias = "us_east_2"
  region = "us-east-2"
  profile = "devnet"
}

provider "aws" {
  alias = "us_east_1"
  region = "us-east-1"
  profile = "devnet"
}

provider "aws" {
  alias = "us_west_2"
  region = "us-west-2"
  profile = "devnet"
}

provider "aws" {
  alias = "eu_central_1"
  region = "eu-central-1"
  profile = "devnet"
}

provider "aws" {
  alias = "ap_southeast_1"
  region = "ap-southeast-1"
  profile = "devnet"
}

# Collect all load balancer DNS names for config file generation
locals {
  load_balancer_dns_mapping = {
    "us-east-1"      = module.ecs_region_lbs_us_east_1.load_balancer_dns_name
    "us-west-2"      = module.ecs_region_lbs_us_west_2.load_balancer_dns_name
    "eu-central-1"   = module.ecs_region_lbs_eu_central_1.load_balancer_dns_name
    "ap-southeast-1" = module.ecs_region_lbs_ap_southeast_1.load_balancer_dns_name
  }
}

# Generate config files using the config_generator module
module "config_generator" {
  source = "./modules/config_generator"

  providers = {
    aws = aws.us_east_2
  }

  app_port                   = var.app_port
  config_bucket_name         = var.config_bucket_name
  config_file_key_prefix     = var.config_file_key_prefix
  regions_config             = var.regions_config
  load_balancer_dns_mapping  = local.load_balancer_dns_mapping

  depends_on = [
    module.ecs_region_lbs_us_east_1,
    module.ecs_region_lbs_us_west_2,
    module.ecs_region_lbs_eu_central_1,
    module.ecs_region_lbs_ap_southeast_1
  ]
}

module "ecs_region_deployments_us_east_1" {
  source = "./modules/ecs_base"

  providers = {
    aws = aws.us_east_1
  }

  region                    = var.regions_config["us-east-1"].aws_region
  app_name                  = var.app_name
  app_image                 = var.app_image
  app_port                  = var.app_port
  config_file_key          = "${var.config_file_key_prefix}${var.regions_config["us-east-1"].config_file_name}"
  config_bucket_name        = var.config_bucket_name
  tcptreaceroute_port      = var.tcptreaceroute_port
  desired_count            = var.desired_count
  load_balancer_addresses  = values(local.load_balancer_dns_mapping)

  depends_on = [module.config_generator]
}

module "ecs_region_deployments_us_west_2" {
  source = "./modules/ecs_base"

  providers = {
    aws = aws.us_west_2
  }

  region                    = var.regions_config["us-west-2"].aws_region
  app_name                  = var.app_name
  app_image                 = var.app_image
  app_port                  = var.app_port
  config_file_key          = "${var.config_file_key_prefix}${var.regions_config["us-west-2"].config_file_name}"
  config_bucket_name        = var.config_bucket_name
  tcptreaceroute_port      = var.tcptreaceroute_port
  desired_count            = var.desired_count
  load_balancer_addresses  = values(local.load_balancer_dns_mapping)

  depends_on = [module.config_generator]
}

module "ecs_region_deployments_eu_central_1" {
  source = "./modules/ecs_base"

  providers = {
    aws = aws.eu_central_1
  }

  region                    = var.regions_config["eu-central-1"].aws_region
  app_name                  = var.app_name
  app_image                 = var.app_image
  app_port                  = var.app_port
  config_file_key          = "${var.config_file_key_prefix}${var.regions_config["eu-central-1"].config_file_name}"
  config_bucket_name        = var.config_bucket_name
  tcptreaceroute_port      = var.tcptreaceroute_port
  desired_count            = var.desired_count
  load_balancer_addresses  = values(local.load_balancer_dns_mapping)

  depends_on = [module.config_generator]
}

module "ecs_region_deployments_ap_southeast_1" {
  source = "./modules/ecs_base"

  providers = {
    aws = aws.ap_southeast_1
  }

  region                    = var.regions_config["ap-southeast-1"].aws_region
  app_name                  = var.app_name
  app_image                 = var.app_image
  app_port                  = var.app_port
  config_file_key          = "${var.config_file_key_prefix}${var.regions_config["ap-southeast-1"].config_file_name}"
  config_bucket_name        = var.config_bucket_name
  tcptreaceroute_port      = var.tcptreaceroute_port
  desired_count            = var.desired_count
  load_balancer_addresses  = values(local.load_balancer_dns_mapping)

  depends_on = [module.config_generator]
}

module "ecs_region_lbs_us_east_1" {
  source = "./modules/load_balancers"

  providers = {
    aws = aws.us_east_1
  }

  region              = var.regions_config["us-east-1"].aws_region
  app_name            = var.app_name
  app_port            = var.app_port
  vpc_cidr_block      = var.regions_config["us-east-1"].vpc_cidr
  tcptreaceroute_port = var.tcptreaceroute_port
}

module "ecs_region_lbs_us_west_2" {
  source = "./modules/load_balancers"

  providers = {
    aws = aws.us_west_2
  }

  region              = var.regions_config["us-west-2"].aws_region
  app_name            = var.app_name
  app_port            = var.app_port
  vpc_cidr_block      = var.regions_config["us-west-2"].vpc_cidr
  tcptreaceroute_port = var.tcptreaceroute_port
}

module "ecs_region_lbs_eu_central_1" {
  source = "./modules/load_balancers"

  providers = {
    aws = aws.eu_central_1
  }

  region              = var.regions_config["eu-central-1"].aws_region
  app_name            = var.app_name
  app_port            = var.app_port
  vpc_cidr_block      = var.regions_config["eu-central-1"].vpc_cidr
  tcptreaceroute_port = var.tcptreaceroute_port
}

module "ecs_region_lbs_ap_southeast_1" {
  source = "./modules/load_balancers"

  providers = {
    aws = aws.ap_southeast_1
  }

  region              = var.regions_config["ap-southeast-1"].aws_region
  app_name            = var.app_name
  app_port            = var.app_port
  vpc_cidr_block      = var.regions_config["ap-southeast-1"].vpc_cidr
  tcptreaceroute_port = var.tcptreaceroute_port
}
