"""Tests for the test doubles in `fakes`."""

import asyncio
import base64
import json
import threading
import unittest

import fakes


class FakeClockTest(unittest.TestCase):
    def test_sleep_advances_and_records(self):
        clock = fakes.FakeClock()
        clock.sleep(2.0)
        asyncio.run(clock.asleep(0.5))
        self.assertEqual((clock.time(), clock.monotonic()), (2.5, 2.5))
        self.assertEqual(clock.sleeps, [2.0, 0.5])

    def test_advancing_past_the_limit_raises(self):
        clock = fakes.FakeClock(limit_s=10.0)
        clock.sleep(10.0)
        with self.assertRaisesRegex(RuntimeError, "advanced past 10 s"):
            clock.sleep(0.1)

    def test_wait_on_a_set_event_does_not_advance(self):
        clock = fakes.FakeClock()
        event = threading.Event()
        event.set()
        self.assertTrue(clock.wait(event, 5.0))
        self.assertEqual((clock.time(), clock.sleeps), (0.0, []))

    def test_wait_returns_whether_the_event_was_set_meanwhile(self):
        event = threading.Event()
        clock = fakes.FakeClock(on_advance=lambda now: event.set())
        self.assertTrue(clock.wait(event, 5.0))
        self.assertEqual(clock.time(), 5.0)
        self.assertFalse(fakes.FakeClock().wait(threading.Event(), 5.0))

    def test_wait_for_times_out_after_advancing(self):
        clock = fakes.FakeClock()
        with self.assertRaises(TimeoutError):
            asyncio.run(clock.wait_for(asyncio.sleep(3600), 2.0))
        self.assertEqual(clock.sleeps, [2.0])


class ScaledClockTest(unittest.TestCase):
    def test_zero_sleep_returns_at_once(self):
        clock = fakes.ScaledClock(50)
        before = clock.time()
        clock.sleep(0)
        asyncio.run(clock.asleep(0))
        self.assertGreaterEqual(clock.time(), before)
        self.assertGreaterEqual(clock.monotonic(), 0.0)

    def test_sleeps_last_the_clock_seconds_asked_for(self):
        clock = fakes.ScaledClock(100)
        before = clock.time()
        clock.sleep(1.0)
        self.assertGreaterEqual(clock.time() - before, 1.0)
        before = clock.time()
        asyncio.run(clock.asleep(1.0))
        self.assertGreaterEqual(clock.time() - before, 1.0)
        event = threading.Event()
        self.assertFalse(clock.wait(event, 1.0))
        self.assertGreaterEqual(clock.time() - before, 2.0)


class FakeNodeTest(unittest.TestCase):
    def setUp(self):
        self.clock = fakes.FakeClock()
        self.url = "http://fake-node"

    def get(self, node, path):
        return node.request("GET", self.url + path)

    def submit(self, node, payload):
        body = json.dumps({"payload": base64.b64encode(payload).decode()}).encode()
        return node.request("POST", self.url + "/v1/submit/submit", body)

    def payload(self, node, height):
        status, body = self.get(node, f"/v1/availability/payload/{height}")
        if status != 200:
            return status, None
        raw = json.loads(body)["data"]["raw_payload"]
        return status, base64.b64decode(raw)

    def test_blocks_are_made_when_a_request_finds_them_due(self):
        node = fakes.FakeNode(self.clock, True)
        self.assertEqual(self.get(node, "/v1/status/block-height"), (200, b"0"))
        self.clock.sleep(2.5 * fakes.BLOCK_S)
        self.assertEqual(self.get(node, "/v1/status/block-height"), (200, b"2"))

    def test_a_submit_lands_in_the_next_block(self):
        node = fakes.FakeNode(self.clock, True)
        self.assertEqual(self.submit(node, b"tx")[0], 200)
        self.assertEqual(node.submits, [0.0])
        self.assertEqual(self.payload(node, 1), (404, None))
        self.clock.sleep(fakes.BLOCK_S)
        self.assertEqual(self.payload(node, 1), (200, b"tx"))

    def test_lost_payload_is_never_served(self):
        node = fakes.FakeNode(self.clock, True, lost=frozenset({1}))
        self.clock.sleep(100.0)
        self.assertEqual(self.payload(node, 1), (404, None))
        self.assertEqual(self.payload(node, 2)[0], 200)

    def test_late_payload_is_served_after_its_delay(self):
        node = fakes.FakeNode(self.clock, True, late={1: 1.0})
        self.clock.sleep(fakes.BLOCK_S)
        self.assertEqual(self.payload(node, 1), (404, None))
        self.clock.sleep(1.0)
        self.assertEqual(self.payload(node, 1)[0], 200)

    def test_query_api_shows_a_block_after_the_lag(self):
        node = fakes.FakeNode(self.clock, True, query_lag=0.3)
        self.clock.sleep(0.5)
        self.assertEqual(self.get(node, "/v1/status/block-height"), (200, b"10"))
        self.assertEqual(self.get(node, "/v1/node/block-height"), (200, b"5"))

    def test_block_takes_at_most_block_txs(self):
        node = fakes.FakeNode(self.clock, True, block_txs=1)
        for tx in (b"a", b"b"):
            self.submit(node, tx)
        self.clock.sleep(2 * fakes.BLOCK_S)
        self.assertEqual(self.payload(node, 1), (200, b"a"))
        self.assertEqual(self.payload(node, 2), (200, b"b"))

    def test_delays_run_on_the_clock(self):
        node = fakes.FakeNode(self.clock, True, accept_delay=0.2, reply_delay=0.3)
        self.submit(node, b"tx")
        self.assertEqual(self.clock.sleeps, [0.2, 0.3])

    def test_unknown_path_raises(self):
        with self.assertRaises(ValueError):
            self.get(fakes.FakeNode(self.clock, True), "/v1/nope")

    def test_pools_close_independently(self):
        node = fakes.FakeNode(self.clock, True)
        one, two = node.connect(self.clock), node.connect(self.clock)
        one.close()
        self.assertTrue(one.closed.is_set())
        self.assertFalse(two.closed.is_set())
        self.assertEqual(
            two.request("GET", self.url + "/v1/status/block-height")[0], 200
        )


class FleetRunnerTest(unittest.TestCase):
    def test_a_matching_pattern_answers_and_the_call_is_recorded(self):
        runner = fakes.FleetRunner([])
        runner.respond("tofu -chdir=x output", lambda argv: fakes.completed("out"))
        self.assertEqual(runner(["tofu", "-chdir=x", "output", "-json"]).stdout, "out")
        self.assertTrue(runner.ran("tofu", "output"))

    def test_the_first_matching_pattern_wins(self):
        runner = fakes.FleetRunner([])
        runner.respond("output", lambda argv: fakes.completed("first"))
        runner.respond("output -json", lambda argv: fakes.completed("second"))
        self.assertEqual(
            runner(["tofu", "-chdir=x", "output", "-json"]).stdout, "first"
        )

    # EDGE:runner-table-no-match
    def test_an_argv_matching_no_pattern_gets_the_default_answer(self):
        runner = fakes.FleetRunner([])
        runner.respond("tofu", lambda argv: fakes.completed("table"))
        self.assertEqual(runner(["git", "rev-parse"]).stdout, "a" * 40 + "\n")
        self.assertEqual(runner(["rsync", "a", "b"]).returncode, 0)

    def test_a_reply_can_extend_the_default_answer(self):
        runner = fakes.FleetRunner([])
        runner.respond(
            "get-resources",
            lambda argv: fakes.completed(
                json.dumps([*json.loads(runner.default(argv).stdout), "arn:extra"])
            ),
        )
        arns = json.loads(runner(["aws", "tagging", "get-resources"]).stdout)
        self.assertEqual(arns, [*fakes.FLEET_ARNS, "arn:extra"])
        self.assertEqual(len(runner.calls), 1)


class FakeRunnerTest(unittest.TestCase):
    def test_an_argv_without_a_matching_prefix_raises(self):
        runner = fakes.FakeRunner({("aws", "sts"): fakes.completed("ok")})
        self.assertEqual(runner(["aws", "sts", "x"]).stdout, "ok")
        with self.assertRaisesRegex(AssertionError, "unexpected command"):
            runner(["aws", "ec2"])
        self.assertTrue(runner.ran("aws", "ec2"))


if __name__ == "__main__":
    unittest.main()
