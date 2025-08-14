# Generate peer configurations for each region
# Each region's config file should contain all OTHER regions as peers
locals {
  peer_configs = {
    for region, config in var.regions_config : region => {
      private_key = config.private_key
      send_mode   = config.send_mode
      message     = config.message
      peers = [
        for peer_region, peer_config in var.regions_config : {
          public_key = peer_config.public_key
          lb_dns     = var.load_balancer_dns_mapping[peer_region]
        }
        # Exclude the current region from its own peer list
        if peer_region != region
      ]
    }
  }
}

# Generate config files using templates
resource "local_file" "config_files" {
  for_each = local.peer_configs

  filename = "config/${each.key}_libp2p_test.toml"
  content = templatefile("${path.module}/templates/libp2p_config.toml.tpl", {
    app_port    = var.app_port
    private_key = each.value.private_key
    peers       = each.value.peers
    send_mode   = each.value.send_mode
    message     = each.value.message
  })
}

# Upload generated config files to S3
resource "aws_s3_object" "generated_config_files" {
  for_each = local.peer_configs

  bucket       = var.config_bucket_name
  key          = "${var.config_file_key_prefix}${each.key}_libp2p_test.toml"
  content      = local_file.config_files[each.key].content
  content_type = "application/toml"

  depends_on = [local_file.config_files]
}
