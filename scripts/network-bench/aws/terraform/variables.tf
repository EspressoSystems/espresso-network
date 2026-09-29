variable "name" {
  type        = string
  description = "Run name; also the tag value for espresso-bench-run."

  validation {
    condition     = can(regex("^[a-z0-9][a-z0-9-]{2,39}$", var.name))
    error_message = "name must match ^[a-z0-9][a-z0-9-]{2,39}$."
  }
}

variable "owner" {
  type = string
}

variable "git_rev" {
  type        = string
  description = "Short git rev of the tree under test; the aws-bench preflight refuses a dirty tree without --allow-dirty, so this is always known."
}

variable "account_id" {
  type        = string
  description = "Expected caller account; the provider refuses to apply against any other."
}

variable "region" {
  type = string
}

variable "profile" {
  type        = string
  description = "AWS CLI profile the provider authenticates with."
}

variable "az" {
  type        = string
  description = "Availability zone `aws-bench preflight` resolved; ignored when offline is true."
}

variable "ami_id" {
  type        = string
  description = "AMI id `aws-bench preflight` resolved; ignored when offline is true."
}

variable "ssh_public_key" {
  type = string
}

variable "operator_cidr" {
  type = string

  validation {
    condition     = can(cidrhost(var.operator_cidr, 0)) && var.operator_cidr != "0.0.0.0/0"
    error_message = "operator_cidr must be a valid CIDR and not 0.0.0.0/0."
  }
}

variable "expires_at" {
  type        = string
  description = "RFC 3339 timestamp; informational only, enforced by each host's own shutdown timer."
}

variable "offline" {
  type        = bool
  default     = false
  description = "Skip every data source and resource that needs real AWS access, for validate/plan without credentials."
}

variable "hosts" {
  description = "name -> host spec. A map with for_each, not a list with count, so removing a host never renumbers the others."
  type = map(object({
    role           = string
    instance_type  = string
    root_gb        = number
    root_iops      = number
    root_mbps      = number
    user_data_path = string
  }))
}
