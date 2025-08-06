listen = "/ip4/0.0.0.0/udp/${app_port}/quic-v1"
private_key = "${private_key}"
peers = [
%{ for peer in peers ~}
    ["${peer.public_key}", "/dns4/${peer.lb_dns}/udp/${app_port}/quic-v1"],
%{ endfor ~}
]
send_mode = ${send_mode}
%{ if message != "" ~}
message = "${message}"
%{ endif ~}
