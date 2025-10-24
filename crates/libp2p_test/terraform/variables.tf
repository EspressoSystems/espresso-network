variable "app_image" {
  description = "Docker image for libp2p test app"
  type        = string
  default     = "ghcr.io/lukaszrzasik/libp2p_test:latest"
}

variable "regions_config" {
  description = "Comprehensive configuration for each region including AWS settings and libp2p node configuration"
  type = map(object({
    aws_region       = string
    config_file_name = string # e.g., "us-east-1_libp2p_test.toml"
    vpc_cidr         = string
    public_key       = string
    private_key      = string
    send_mode        = bool
    message          = string
  }))
  default = {
    "us-east-1" = {
      aws_region       = "us-east-1"
      config_file_name = "us-east-1_libp2p_test.toml"
      vpc_cidr         = "10.0.0.0/16"
      public_key       = "12D3KooWKRd4rDf2iKjudQfzH2rT8P629XFMUx6P2PQQpJxPcDHi"
      private_key      = "BLS_SIGNING_KEY~I8wAPjR0ksgO01wreQ9XgJsPXoCTmBylEleQfAbI1RbN"
      send_mode        = true
      message          = "test-message"
    },
    "us-west-2" = {
      aws_region       = "us-west-2"
      config_file_name = "us-west-2_libp2p_test.toml"
      vpc_cidr         = "10.1.0.0/16"
      public_key       = "12D3KooWFzobBRYtkR1G4g4hcwEpriKG99R7QqSJYiRH1kb5wjkY"
      private_key      = "BLS_SIGNING_KEY~qruBWMuMtZ5yd1FoO4_e5yULswa7HFNz0Y_fpbOpVBeP"  # Update with actual key
      send_mode        = false
      message          = ""
    },
    "eu-central-1" = {
      aws_region       = "eu-central-1"
      config_file_name = "eu-central-1_libp2p_test.toml"
      vpc_cidr         = "10.2.0.0/16"
      public_key       = "12D3KooWGZJippUFtCCSGKDt7hxXspAiM2EZb5KSUaGWN5iFP45w"
      private_key      = "BLS_SIGNING_KEY~b-5gZNRpYlkRyBm_cpAEqqXHCF9C1KaJk_5AL80W0Snr"
      send_mode        = false
      message          = ""
    },
    "ap-southeast-1" = {
      aws_region       = "ap-southeast-1"
      config_file_name = "ap-southeast-1_libp2p_test.toml"
      vpc_cidr         = "10.3.0.0/16"
      public_key       = "12D3KooWAPho9bSmLeyPSL4zypbPKxKjAJPpSUQ7KN6zG4Jkr5dx"
      private_key      = "BLS_SIGNING_KEY~KoDNTepis6rPPvEinJ_AXiCY0BxguTzlx9hd3LYFDQvk"  # Update with actual key
      send_mode        = false
      message          = ""
    }
  }
}

variable "app_port" {
  description = "Port your application listens on"
  type        = number
  default     = 9000 # Example port for libp2p, adjust as needed
}

variable "config_bucket_name" {
  description = "Name for the S3 bucket where config files are stored"
  type        = string
  default     = "cuiuwzcvojq6cywlii8s2kjqwi5hapibgwbp5wnnldxxjzb1n9" # Existing bucket
}

variable "app_name" {
  description = "A name for your application, used in resource naming"
  type        = string
  default     = "libp2p-test-app"
}

variable "config_file_key_prefix" {
  description = "Key prefix for the S3 bucket where config files are stored"
  type        = string
  default     = "libp2p_test_configs/"
}

variable "tcptreaceroute_port" {
  description = "tcptraceroute port for baseline measurements"
  type        = number
  default     = 10000
}

variable "desired_count" {
  description = "Number of instances to run in each region. Set to 0 for quick service shutdown"
  type        = number
  default     = 1
}