variable "region" {
  type        = string
  description = "AWS region for this deployment"
}
variable "app_name" {
  type        = string
  description = "Name for the application"
}
variable "app_port" {
  type        = number
  description = "Port the application listens on"
}
variable "vpc_cidr_block" {
  type        = string
  description = "CIDR block for the VPC in this region"
}
variable "tcptreaceroute_port" {
  description = "tcptraceroute port for baseline measurements"
  type        = number
}

# Data source for current AWS account ID
data "aws_caller_identity" "current" {}

resource "aws_vpc" "libp2p_test" {
  cidr_block = var.vpc_cidr_block
  enable_dns_hostnames = true
  enable_dns_support   = true
  tags = {
    Name = "${var.app_name}-${var.region}-vpc"
  }
}

resource "aws_internet_gateway" "libp2p_gw" {
  vpc_id = aws_vpc.libp2p_test.id
  tags = {
    Name = "${var.app_name}-${var.region}-igw"
  }
}

resource "aws_subnet" "libp2p_public" {
  count             = 1
  vpc_id            = aws_vpc.libp2p_test.id
  cidr_block        = cidrsubnet(aws_vpc.libp2p_test.cidr_block, 8, count.index) # Example: 10.X.0.0/24, 10.X.1.0/24
  availability_zone = data.aws_availability_zones.available.names[count.index]
  map_public_ip_on_launch = true
  tags = {
    Name = "${var.app_name}-${var.region}-subnet-${count.index}"
  }
}

resource "aws_route_table" "libp2p_public_rt" {
  vpc_id = aws_vpc.libp2p_test.id
  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.libp2p_gw.id
  }
  tags = {
    Name = "${var.app_name}-${var.region}-public-rt"
  }
}

resource "aws_route_table_association" "libp2p_public" {
  count          = length(aws_subnet.libp2p_public)
  subnet_id      = aws_subnet.libp2p_public[count.index].id
  route_table_id = aws_route_table.libp2p_public_rt.id
}

resource "aws_security_group" "libp2p_sg" {
  vpc_id      = aws_vpc.libp2p_test.id
  name        = "${var.app_name}-${var.region}-ecs-tasks-sg"
  description = "Allow inbound traffic to LB and outbound to anywhere"

  # Inbound for your application port (e.g., libp2p port)
  ingress {
    from_port   = var.app_port
    to_port     = var.app_port
    protocol    = "udp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # Inbound for health check
  ingress {
    from_port   = var.app_port
    to_port     = var.app_port
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # Inbound for tcptraceroute
  ingress {
    from_port   = var.tcptreaceroute_port
    to_port     = var.tcptreaceroute_port
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # Allow all outbound traffic (Fargate needs this to pull images, talk to S3, CloudWatch, etc.)
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${var.app_name}-${var.region}-ecs-tasks-sg"
  }
}

resource "aws_lb" "libp2p_network" {
  name                             = "${var.app_name}-network"
  load_balancer_type               = "network"
  enable_cross_zone_load_balancing = true
  security_groups                  = [aws_security_group.libp2p_sg.id]
  subnets                          = aws_subnet.libp2p_public.*.id
  idle_timeout                     = 300
  lifecycle {
    ignore_changes = [
      access_logs
    ]
  }
}

resource "aws_lb_listener" "libp2p" {
  load_balancer_arn = aws_lb.libp2p_network.id
  port              = var.app_port
  protocol          = "UDP"
  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.libp2p.arn
  }
}

resource "aws_lb_listener" "tcptraceroute" {
  load_balancer_arn = aws_lb.libp2p_network.id
  port              = var.tcptreaceroute_port
  protocol          = "TCP"
  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.tcptraceroute.arn
  }
}

resource "aws_lb_target_group" "libp2p" {
  name                   = "${var.app_name}-${var.region}"
  connection_termination = true
  deregistration_delay   = 1
  port                   = var.app_port
  protocol               = "UDP"
  target_type            = "ip"
  vpc_id                 = aws_vpc.libp2p_test.id
}

resource "aws_lb_target_group" "tcptraceroute" {
  name                   = "tcptraceroute-${var.region}"
  connection_termination = true
  deregistration_delay   = 1
  port                   = var.tcptreaceroute_port
  protocol               = "TCP"
  target_type            = "ip"
  vpc_id                 = aws_vpc.libp2p_test.id
}

data "aws_availability_zones" "available" {
  state = "available"
}

# Output the load balancer DNS name
output "load_balancer_dns_name" {
  description = "DNS name of the load balancer"
  value       = aws_lb.libp2p_network.dns_name
}