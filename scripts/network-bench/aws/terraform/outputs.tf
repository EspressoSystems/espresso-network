output "hosts" {
  value = {
    for k, i in aws_instance.host : k => {
      role        = var.hosts[k].role
      public_ip   = i.public_ip
      private_ip  = i.private_ip
      private_dns = i.private_dns
      instance_id = i.id
    }
  }
}

output "az" {
  value = var.az
}

output "ami_id" {
  value = var.ami_id
}

output "security_group_id" {
  value = aws_security_group.this.id
}

output "pg_volume_id" {
  description = "Volume id of the query host's Postgres volume; null without one."
  value = one(flatten([
    for k, i in aws_instance.host : [for d in i.ebs_block_device : d.volume_id]
    if var.hosts[k].role == "query"
  ]))
}

output "rds" {
  value = length(aws_db_instance.this) == 0 ? null : {
    identifier     = aws_db_instance.this[0].identifier
    endpoint       = aws_db_instance.this[0].address
    port           = aws_db_instance.this[0].port
    resource_id    = aws_db_instance.this[0].resource_id
    arn            = aws_db_instance.this[0].arn
    engine_version = aws_db_instance.this[0].engine_version_actual
    schedule_group = aws_scheduler_schedule_group.this[0].name
    schedule_name  = aws_scheduler_schedule.rds_delete[0].name
  }
}
