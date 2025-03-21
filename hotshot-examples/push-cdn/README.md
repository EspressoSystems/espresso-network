Steps
---------------

KeyDB is the ephemeral database, it's like Redis but with extra features. The only thing we run it with is with `--requirepass` to set a password.

**Marshals:**
The marshal is the entry point of the push CDN, all users connect there first. It tells users which broker to connect to.

- `-d` is the "discovery endpoint", which in this case is the URL of KeyDB.
- `-b` is the bind port. This is what you would set in run_config.toml for cdn_broker_marshal_endpoint
- `-m` is metrics stuff. You shouldn't have to use that


**Brokers:**
In a run with multiple machines, we want two brokers. With one machine, it's probably fine to do one broker. These are what route the messages. Here are the relevant command line arguments:

- `-d` is the "discovery endpoint", which in this case is the URL of KeyDB.
- `--public-bind-endpoint`: the endpoint which we bind to locally for users to connect to (e.g. 0.0.0.0:1740)
- `--public-advertise-endpoint`: the endpoint which we advertise to users (e.g. my.public.ip:1740)
- `--private-bind-endpoint`: the endpoint which we bind to locally for brokers to connect to (e.g. 0.0.0.0:1741)
- `--private-advertise-endpoint`: the endpoint which we advertise to brokers (e.g. my.public.ip:1741)
- `-m` is metrics stuff. You shouldn't have to use that
For brokers, there is a magic value called `local_ip`. This resolves to the local IP address, which skips the need for talking to the AWS metadata server. For in-AWS uses, the following configuration is probably fine:
`cdn-broker --public-bind-endpoint 0.0.0.0:1740 --public-advertise-endpoint local_ip:1740 --private-bind-endpoint 0.0.0.0:1741 --private-advertise-endpoint local_ip:1741`. You won't need to put this port or values anywhere, as the marshal does everything for you.

Examples:
---------------

**Run Locally** 

`just example all-push-cdn -- --config_file ./crates/orchestrator/run-config.toml`

OR

```
docker run --rm -p 0.0.0.0:6379:6379 eqalpha/keydb
just example cdn-marshal -- -d redis://localhost:6379 -b 9000
just example cdn-broker -- -d redis://localhost:6379 --public-bind-endpoint 0.0.0.0:1740 --public-advertise-endpoint local_ip:1740 --private-bind-endpoint 0.0.0.0:1741 --private-advertise-endpoint local_ip:1741
just example orchestrator -- --config_file ./crates/orchestrator/run-config.toml --orchestrator_url http://0.0.0.0:4444
just example multi-validator-push-cdn -- 10 http://127.0.0.1:4444
```

**Run with GPU-VID** 
```
docker run --rm -p 0.0.0.0:6379:6379 eqalpha/keydb
just example cdn-marshal -- -d redis://localhost:6379 -b 9000
just example cdn-broker -- -d redis://localhost:6379 --public-bind-endpoint 0.0.0.0:1740 --public-advertise-endpoint local_ip:1740 --private-bind-endpoint 0.0.0.0:1741 --private-advertise-endpoint local_ip:1741
just example_fixed_leader orchestrator -- --config_file ./crates/orchestrator/run-config.toml --orchestrator_url http://0.0.0.0:4444 --fixed_leader_for_gpuvid 1
just example_gpuvid_leader multi-validator-push-cdn -- 1 http://127.0.0.1:4444
sleep 1m
just example_fixed_leader multi-validator-push-cdn -- 9 http://127.0.0.1:4444
```

Where ones using `example_gpuvid_leader` could be the leader and should be running on an nvidia GPU, and other validators using `example_fixed_leader` will never be a leader. In practice, these url should be changed to the corresponding ip and port.


If you don't have a gpu but want to test out fixed leader, you can run:
```
docker run --rm -p 0.0.0.0:6379:6379 eqalpha/keydb
just example cdn-marshal -- -d redis://localhost:6379 -b 9000
just example cdn-broker -- -d redis://localhost:6379 --public-bind-endpoint 0.0.0.0:1740 --public-advertise-endpoint local_ip:1740 --private-bind-endpoint 0.0.0.0:1741 --private-advertise-endpoint local_ip:1741
just example_fixed_leader orchestrator -- --config_file ./crates/orchestrator/run-config.toml --orchestrator_url http://0.0.0.0:4444 --fixed_leader_for_gpuvid 1
just example_fixed_leader multi-validator-push-cdn -- 1 http://127.0.0.1:4444
sleep 1m
just example_fixed_leader multi-validator-push-cdn -- 9 http://127.0.0.1:4444
```

Remember, you have to run leaders first, then other validators, so that leaders will have lower index.
