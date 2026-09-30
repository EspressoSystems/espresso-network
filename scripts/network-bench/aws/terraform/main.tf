# offline=true skips every data source and real resource, so validate/plan work with fake
# credentials; the driver never sets it for a real run.
provider "aws" {
  region  = var.region
  profile = var.offline ? null : var.profile

  allowed_account_ids = var.offline ? null : [var.account_id]

  skip_credentials_validation = var.offline
  skip_requesting_account_id  = var.offline
  skip_metadata_api_check     = var.offline

  default_tags {
    tags = {
      espresso-bench-run     = var.name
      espresso-bench-owner   = var.owner
      espresso-bench-expires = var.expires_at
      espresso-bench-git     = var.git_rev
    }
  }
}

# az and ami_id come from `aws-bench preflight`, which already picked an AZ offering every
# requested instance type and resolved the arm64 AMI; this module only looks up that AZ's
# default subnet.
locals {
  ami_id    = var.offline ? "ami-0offline00000000" : var.ami_id
  pinned_az = var.offline ? "eu-west-1a" : var.az
}

data "aws_vpc" "default" {
  count   = var.offline ? 0 : 1
  default = true
}

data "aws_subnets" "default" {
  count = var.offline ? 0 : 1

  filter {
    name   = "vpc-id"
    values = [data.aws_vpc.default[0].id]
  }
  filter {
    name   = "availability-zone"
    values = [var.az]
  }
  filter {
    name   = "default-for-az"
    values = ["true"]
  }
}

locals {
  pinned_subnet = var.offline ? "subnet-0offline0000000" : (
    length(data.aws_subnets.default[0].ids) > 0 ? data.aws_subnets.default[0].ids[0] : ""
  )
}

resource "aws_key_pair" "this" {
  key_name   = "espresso-bench-${var.name}"
  public_key = var.ssh_public_key
}

# No API port is opened: every HTTP service runs on ctl and the driver reads it over ssh.
resource "aws_security_group" "this" {
  name        = "espresso-bench-${var.name}"
  description = "espresso-bench fleet ${var.name}"
  vpc_id      = var.offline ? null : data.aws_vpc.default[0].id

  ingress {
    description = "ssh from the operator"
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = [var.operator_cidr]
  }

  ingress {
    description = "all traffic within the fleet"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    self        = true
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_instance" "host" {
  for_each = var.hosts

  ami                    = local.ami_id
  instance_type          = each.value.instance_type
  key_name               = aws_key_pair.this.key_name
  vpc_security_group_ids = [aws_security_group.this.id]
  subnet_id              = local.pinned_subnet

  associate_public_ip_address          = true
  instance_initiated_shutdown_behavior = "terminate"

  metadata_options {
    http_tokens = "required"
  }

  user_data                   = file(each.value.user_data_path)
  user_data_replace_on_change = true

  root_block_device {
    volume_type           = "gp3"
    volume_size           = each.value.root_gb
    iops                  = each.value.root_iops
    throughput            = each.value.root_mbps
    delete_on_termination = true

    # Explicit because default_tags do not reliably reach root volumes.
    tags = {
      Name                = "${var.name}-${each.key}-root"
      espresso-bench-run  = var.name
      espresso-bench-role = each.value.role
    }
  }

  # Inline rather than aws_ebs_volume plus attachment: the volume is deleted with the instance
  # at the shutdown timer, so an abandoned fleet leaves no volume behind.
  dynamic "ebs_block_device" {
    for_each = each.value.role == "query" && var.pg_volume != null ? [var.pg_volume] : []

    content {
      device_name           = "/dev/sdf"
      volume_type           = "gp3"
      volume_size           = ebs_block_device.value.gb
      iops                  = ebs_block_device.value.iops
      throughput            = ebs_block_device.value.mbps
      delete_on_termination = true

      tags = {
        Name                = "${var.name}-${each.key}-pg"
        espresso-bench-run  = var.name
        espresso-bench-role = "pg"
      }
    }
  }

  tags = {
    Name                = "${var.name}-${each.key}"
    espresso-bench-role = each.value.role
  }

  lifecycle {
    precondition {
      condition     = var.offline || local.pinned_subnet != ""
      error_message = "No default subnet for ${var.az} in ${var.region}."
    }
  }
}
