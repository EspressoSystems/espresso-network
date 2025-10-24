### Description

A very simple application that spawns three receivers and one sender.
The instances use libp2p-networking library for sending direct messages between them.
The sender sends a message to each receiver and then waits for the responses.
The round trip time is reported in the logs.

### Usage in local containers

1. Create a release build:
`cargo build -p libp2p_test --release`

2. Build docker a image:
`docker build -f crates/libp2p_test/Dockerfile -t libp2p_test .`

3. Create an internal docker network:
`docker network create libp2p_test_network`

4. Run docker containers (wait 10 seconds for receivers to exit):
`for i in $(ls crates/libp2p_test/config); do docker run -d --network libp2p_network -v $PWD/crates/libp2p_test/config/$i/libp2p_test.toml:/app_config/libp2p_test.toml --name $i libp2p_test; done`

5. The test runs indefinitely. Stop it after a while:
`for i in $(ls crates/libp2p_test/config/); do docker stop $i; done`

6. Grab the logs for analysis:
`for i in $(ls crates/libp2p_test/config/); do docker logs $i &> $i; done`

6. Remove finished containers:
`for i in $(ls crates/libp2p_test/config/); do docker rm $i ; done`

7. (Optionally) Remove the docker image:
`docker rmi libp2p_test`

8. (Optionally) Remove the internal docker network:
`docker network rm libp2p_test_network`

### Usage with terraform and AWS

Terraform assumes access to the AWS devnet profile.
Terraform assumes an existing S3 bucket with the name `cuiuwzcvojq6cywlii8s2kjqwi5hapibgwbp5wnnldxxjzb1n9` in us-east-2 region.

## Terraform plan

Terraform executes several steps:
1. Deploys a load balancer in each region. This is required so that the libp2p_test app can peer with known addresses.
2. Generates a config file for each libp2p_test instance in each region.
3. Uploads the config files to S3.
4. Deploys libp2p_test instances in each region.
5. Deploys tcptraceroute instances in each region. These are treated as baseline measurements.

## Adding new regions

Adding a new region requires the following steps:
1. Add a new entry to `regions_config` in `variables.tf`
2. Add a new entry to `load_balancer_dns_mapping` in `main.tf`
3. Add a new `load_balancer` module call in `main.tf`
4. Add a new `ecs_base` module call in `main.tf`
5. Add a new aws provider with an alias in `main.tf`

## Running terraform

1. Initialize terraform:
`terraform init`

2. Plan the deployment:
`terraform plan -out libp2p_test`

3. Apply the deployment:
`terraform apply "libp2p_test"`

4. Plan the destruction:
`terraform plan -destroy -out libp2p_test_destroy`

5. Destroy the deployment:
`terraform destroy "libp2p_test_destroy"`