sudo nix develop -c flamegraph -- $(fd -I "counter*" -t x | rg debug) test_request_response_one_round
