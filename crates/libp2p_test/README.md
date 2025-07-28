### Description

A very simple application that spawns three receivers and one sender.
The instances use libp2p-networking library for sending direct messages between them.
The sender sends a message to each receiver and then waits for the responses.
The round trip time is reported in the logs.

### Usage in local containers

1. Create a release build:
`cargo build -p libp2p_test --release`

2. Build docker images:
`for i in $(ls crates/libp2p_test/config/); do echo $i; docker build --build-arg CONFIG=$i -t $i -f crates/libp2p_test/Dockerfile . ; done`

3. Create an internal docker network:
`docker network create libp2p_test_network`

4. Run docker containers (wait 10 seconds for receivers to exit):
`for i in $(ls crates/libp2p_test/config/); do echo $i; docker run -d --name $i --network libp2p_test_network $i; done`

5. Grab the logs for analysis:
`for i in $(ls crates/libp2p_test/config/); do echo $i; docker logs $i &> $i; done`

6. Remove finished containers:
`for i in $(ls crates/libp2p_test/config/); do echo $i; docker rm $i ; done`

7. (Optionally) Remove docker images:
`for i in $(ls crates/libp2p_test/config/); do echo $i; docker rmi $i ; done`

8. (Optionally) Remove the internal docker network:
`docker network rm libp2p_test_network`
