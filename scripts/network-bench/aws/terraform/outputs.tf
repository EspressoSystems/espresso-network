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
  value = local.pinned_az
}

output "ami_id" {
  value = local.ami_id
}

output "security_group_id" {
  value = aws_security_group.this.id
}
