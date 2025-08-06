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
