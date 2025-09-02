variable "app_port" {
  type        = number
  description = "Port the application listens on"
}

variable "config_bucket_name" {
  type        = string
  description = "S3 bucket name for storing config files"
}

variable "config_file_key_prefix" {
  type        = string
  description = "S3 key prefix for config files"
}

variable "regions_config" {
  type = map(object({
    aws_region       = string
    config_file_name = string
    vpc_cidr         = string
    public_key       = string
    private_key      = string
    send_mode        = bool
    message          = string
  }))
  description = "Comprehensive configuration for each region including AWS settings and libp2p node configuration"
}

variable "load_balancer_dns_mapping" {
  type        = map(string)
  description = "Mapping of region names to load balancer DNS names"
}
