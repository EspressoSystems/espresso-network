# modules/ecs_base/main.tf

variable "region" {
  type        = string
  description = "AWS region for this deployment"
}
variable "app_name" {
  type        = string
  description = "Name for the application"
}
variable "app_image" {
  type        = string
  description = "Docker image for the application"
}
variable "app_port" {
  type        = number
  description = "Port the application listens on"
}

variable "config_bucket_name" {
  type        = string
  description = "Name of the S3 bucket for config files"
}

variable "config_file_key" {
  type        = string
  description = "Key of the S3 object for config file"
}

variable "tcptreaceroute_port" {
  description = "tcptraceroute port for baseline measurements"
  type        = number
}

variable "desired_count" {
  description = "Number of instances to run in each region"
  type        = number
}

variable "load_balancer_addresses" {
  description = "List of load balancer DNS addresses for tcptraceroute baseline measurements"
  type        = list(string)
}

# Data source for current AWS account ID
data "aws_caller_identity" "current" {}

data "aws_subnet" "libp2p_public" {
  count = 1
  filter {
    name   = "tag:Name"
    values = ["${var.app_name}-${var.region}-subnet-${count.index}"]
  }
}

data "aws_security_group" "libp2p_sg" {
  filter {
    name   = "tag:Name"
    values = ["${var.app_name}-${var.region}-ecs-tasks-sg"]
  }
}

data "aws_lb_target_group" "libp2p" {
  name = "${var.app_name}-${var.region}"
}

data "aws_lb_target_group" "tcptraceroute" {
  name = "tcptraceroute-${var.region}"
}

# --- ECS Cluster ---
resource "aws_ecs_cluster" "libp2p_test" {
  name = "${var.app_name}-${var.region}-cluster"

  tags = {
    Name = "${var.app_name}-${var.region}-cluster"
  }
}

# --- IAM Roles for ECS ---
resource "aws_iam_role" "libp2p_ecs_task_execution_role" {
  name = "${var.app_name}-${var.region}-ecs-task-execution-role"
  assume_role_policy = jsonencode({
    Version = "2012-10-17",
    Statement = [{
      Action = "sts:AssumeRole",
      Effect = "Allow",
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
    }]
  })
  tags = {
    Name = "${var.app_name}-${var.region}-ecs-task-execution-role"
  }
}

resource "aws_iam_role_policy_attachment" "libp2p_ecs_task_execution_role_policy" {
  role       = aws_iam_role.libp2p_ecs_task_execution_role.id
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

# Policy to allow the ECS Task Execution Role to read from the S3 config bucket
resource "aws_iam_role_policy" "libp2p_s3_config_read_policy" {
  name = "${var.app_name}-${var.region}-s3-config-read-policy"
  role = aws_iam_role.libp2p_ecs_task_execution_role.id

  policy = jsonencode({
    Version = "2012-10-17",
    Statement = [
      {
        Effect = "Allow",
        Action = [
          "s3:GetObject"
        ],
        Resource = "arn:aws:s3:::${var.config_bucket_name}/${var.config_file_key}"
      },
      {
        Effect = "Allow",
        Action = [
          "s3:ListBucket"
        ],
        Resource = "arn:aws:s3:::${var.config_bucket_name}"
      }
    ]
  })
}


# --- ECS Task Definition with Init Container for Config File ---
resource "aws_ecs_task_definition" "libp2p_app_task" {
  family                   = "${var.app_name}-${var.region}-task"
  cpu                      = "512" # Fargate CPU units
  memory                   = "1024" # Fargate Memory units
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  execution_role_arn       = aws_iam_role.libp2p_ecs_task_execution_role.arn
  task_role_arn            = aws_iam_role.libp2p_ecs_task_execution_role.arn # Can be a separate role if app needs AWS APIs

  # Define a volume for the config file to be shared between init and main container
  volume {
    name = "config-volume"
  }

  container_definitions = jsonencode([
    # Init Container: Downloads the config file from S3
    {
      name        = "init-config-downloader"
      image       = "alpine:latest"
      essential   = false
      command = [
        "sh", "-c",
        "apk add --no-cache aws-cli && if [ ! -f /app_config/libp2p_test.toml ]; then echo 'Config file not found, downloading from S3...' >&2 && aws s3 cp s3://${var.config_bucket_name}/${var.config_file_key} /app_config/libp2p_test.toml; else echo 'Config file already exists, skipping download.' >&2; fi && sleep 60"
      ]
      mountPoints = [
        {
          sourceVolume  = "config-volume"
          containerPath = "/app_config" # Mount shared volume here
        }
      ]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = "/ecs/${var.app_name}/init"
          "awslogs-region"        = var.region
          "awslogs-stream-prefix" = "init-config"
        }
      }
    },
    # Health check container: listens on TCP port
    {
      name        = "health-check-listener"
      image       = "alpine:latest"
      essential   = true
      command     = [
        "sh", "-c",
        "apk add --no-cache netcat-openbsd && nc -lkp ${var.app_port}"
      ]
      portMappings = [{
        containerPort = var.app_port
        hostPort      = var.app_port
        protocol      = "tcp"
      }]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = "/ecs/${var.app_name}/health"
          "awslogs-region"        = var.region
          "awslogs-stream-prefix" = "health-check"
        }
      }
    },
    # Main Application Container
    {
      name      = var.app_name
      image     = var.app_image
      essential = true
      portMappings = [
        {
          containerPort = var.app_port
          hostPort = var.app_port # Fargate uses hostPort = containerPort
          protocol      = "udp"
        }
      ]
      mountPoints = [
        {
          sourceVolume  = "config-volume"
          containerPath = "/app_config" # Mount shared volume here
        }
      ]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = "/ecs/${var.app_name}/app"
          "awslogs-region"        = var.region
          "awslogs-stream-prefix" = "app"
        }
      }
    },
  ])
  tags = {
    Name = "${var.app_name}-${var.region}-task-def"
  }
}

# --- ECS Task Definition for tcptraceroute ---
resource "aws_ecs_task_definition" "tcptraceroute_task" {
  family                   = "tcptraceroute-${var.region}-task"
  cpu                      = "512" # Fargate CPU units
  memory                   = "1024" # Fargate Memory units
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  execution_role_arn       = aws_iam_role.libp2p_ecs_task_execution_role.arn
  task_role_arn            = aws_iam_role.libp2p_ecs_task_execution_role.arn # Can be a separate role if app needs AWS APIs

  container_definitions = jsonencode([
    # tcptraceroute baseline
    {
      name      = "tcptraceroute-baseline"
      image     = "alpine:latest"
      essential = true
      command = [
        "sh", "-c",
        "apk add --no-cache tcptraceroute && while true; do for i in ${join(" ", var.load_balancer_addresses)}; do echo -ne \"\n\n###### $i ######\n\n\"; tcptraceroute $i ${var.tcptreaceroute_port}; done; sleep 1; done"
      ]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = "/ecs/tcptraceroute"
          "awslogs-region"        = var.region
          "awslogs-stream-prefix" = "tcptraceroute"
        }
      }
    },
    # tcptraceroute listener
    {
      name        = "tcptraceroute-listener"
      image       = "alpine:latest"
      essential   = true
      command     = [
        "sh", "-c",
        "apk add --no-cache netcat-openbsd && nc -lkp ${var.tcptreaceroute_port}"
      ]
      portMappings = [{
        containerPort = var.tcptreaceroute_port
        hostPort      = var.tcptreaceroute_port
        protocol      = "tcp"
      }]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = "/ecs/tcptraceroute-listener"
          "awslogs-region"        = var.region
          "awslogs-stream-prefix" = "tcptraceroute-listener"
        }
      }
    },
  ])
  tags = {
    Name = "tcptraceroute-${var.region}-task-def"
  }
}

# --- CloudWatch Log Group for ECS Task Logs ---
resource "aws_cloudwatch_log_group" "libp2p_app_logs" {
  name              = "/ecs/${var.app_name}/app" # Logs for the main app
  retention_in_days = 7 # Adjust as needed
  tags = {
    Name = "${var.app_name}-${var.region}-app-logs"
  }
}

resource "aws_cloudwatch_log_group" "libp2p_init_logs" {
  name              = "/ecs/${var.app_name}/init" # Logs for the init container
  retention_in_days = 7
  tags = {
    Name = "${var.app_name}-${var.region}-init-logs"
  }
}

resource "aws_cloudwatch_log_group" "libp2p_health_check" {
  name              = "/ecs/${var.app_name}/health" # Logs for the health check container
  retention_in_days = 7
  tags = {
    Name = "${var.app_name}-${var.region}-health-logs"
  }
}

resource "aws_cloudwatch_log_group" "libp2p_tcptraceroute" {
  name              = "/ecs/tcptraceroute" # Logs for tcptraceroute
  retention_in_days = 7
  tags = {
    Name = "tcptraceroute-${var.region}-logs"
  }
}

resource "aws_cloudwatch_log_group" "libp2p_tcptraceroute_listener" {
  name              = "/ecs/tcptraceroute-listener" # Logs for tcptraceroute listener
  retention_in_days = 7
  tags = {
    Name = "tcptraceroute-listener-${var.region}-logs"
  }
}

# --- ECS Service ---
resource "aws_ecs_service" "app_service" {
  name            = "${var.app_name}-${var.region}-service"
  cluster         = aws_ecs_cluster.libp2p_test.id
  task_definition = aws_ecs_task_definition.libp2p_app_task.arn
  desired_count   = var.desired_count # set to 0 for quick service shutdown
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = data.aws_subnet.libp2p_public.*.id
    security_groups  = [data.aws_security_group.libp2p_sg.id]
    assign_public_ip = true
  }

  load_balancer {
    target_group_arn = data.aws_lb_target_group.libp2p.arn
    container_name   = var.app_name
    container_port   = var.app_port
  }

  # Optional: For graceful deployments
  deployment_controller {
    type = "ECS"
  }

  tags = {
    Name = "${var.app_name}-${var.region}-service"
  }
}

resource "aws_ecs_service" "tcptraceroute_service" {
  name            = "tcptraceroute-${var.region}-service"
  cluster         = aws_ecs_cluster.libp2p_test.id
  task_definition = aws_ecs_task_definition.tcptraceroute_task.arn
  desired_count   = var.desired_count # set to 0 for quick service shutdown
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = data.aws_subnet.libp2p_public.*.id
    security_groups  = [data.aws_security_group.libp2p_sg.id]
    assign_public_ip = true
  }

  load_balancer {
    target_group_arn = data.aws_lb_target_group.tcptraceroute.arn
    container_name   = "tcptraceroute-listener"
    container_port   = var.tcptreaceroute_port
  }

  # Optional: For graceful deployments
  deployment_controller {
    type = "ECS"
  }

  tags = {
    Name = "tcptraceroute-${var.region}-service"
  }
}
