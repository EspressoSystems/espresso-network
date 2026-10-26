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
    engine_version = aws_db_instance.this[0].engine_version_actual
  }
}
