"""unittest runner that prints the slowest tests and fails on a slow one.

Only a single test over `--max-test-s` fails the run. The suite total is advisory.
"""

import argparse
import sys
import time
import unittest
from typing import Any


class TimedResult(unittest.TextTestResult):
    slowest_n = 10

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        super().__init__(*args, **kwargs)
        self.timings: list[tuple[float, str]] = []
        self._started = 0.0

    def startTest(self, test: unittest.TestCase) -> None:
        super().startTest(test)
        self._started = time.perf_counter()

    def stopTest(self, test: unittest.TestCase) -> None:
        self.timings.append((time.perf_counter() - self._started, test.id()))
        super().stopTest(test)

    def slowest(self, n: int) -> list[tuple[float, str]]:
        return sorted(self.timings, reverse=True)[:n]

    def over_limit(self, limit_s: float) -> list[tuple[float, str]]:
        return [(s, name) for s, name in self.slowest(len(self.timings)) if s > limit_s]

    def printErrors(self) -> None:
        super().printErrors()
        for seconds, name in self.slowest(self.slowest_n):
            self.stream.writeln(f"{seconds:6.2f}s  {name}")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("-s", "--start-directory", default=".")
    parser.add_argument("-p", "--pattern", default="test_*.py")
    parser.add_argument(
        "-k", dest="names", action="append", default=[], metavar="SUBSTR"
    )
    parser.add_argument("--slowest", type=int, default=10)
    parser.add_argument("--max-test-s", type=float)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    loader = unittest.TestLoader()
    if args.names:
        loader.testNamePatterns = [n if "*" in n else f"*{n}*" for n in args.names]
    suite = loader.discover(args.start_directory, pattern=args.pattern)

    TimedResult.slowest_n = args.slowest
    result = unittest.TextTestRunner(resultclass=TimedResult, stream=sys.stderr).run(
        suite
    )
    assert isinstance(result, TimedResult)

    if args.max_test_s is not None:
        slow = result.over_limit(args.max_test_s)
        for seconds, name in slow:
            print(
                f"test {name} took {seconds:.1f} s, limit {args.max_test_s:g} s",
                file=sys.stderr,
            )
        if slow:
            return 1
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
