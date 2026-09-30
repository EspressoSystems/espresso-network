import contextlib
import io
import itertools
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

import timed_tests


def result_with(durations: list[tuple[float, str]]) -> timed_tests.TimedResult:
    result = timed_tests.TimedResult(io.StringIO(), descriptions=False, verbosity=0)
    result.timings = durations
    return result


class OverLimitTest(unittest.TestCase):
    def test_equal_to_the_limit_passes(self):
        self.assertEqual(result_with([(1.0, "a")]).over_limit(1.0), [])

    def test_strictly_greater_fails(self):
        result = result_with([(1.0, "a"), (1.5, "b"), (0.2, "c")])
        self.assertEqual(result.over_limit(1.0), [(1.5, "b")])

    def test_slowest_is_sorted_and_truncated(self):
        result = result_with([(0.1, "a"), (0.3, "b"), (0.2, "c")])
        self.assertEqual(result.slowest(2), [(0.3, "b"), (0.2, "c")])


class MainTest(unittest.TestCase):
    sample_ids = itertools.count()

    def run_main(self, *extra: str) -> tuple[int, str]:
        body = """
            import time
            import unittest

            class T(unittest.TestCase):
                def test_slow(self):
                    time.sleep(0.02)

                def test_fast(self):
                    pass
        """
        with tempfile.TemporaryDirectory() as tmp:
            module = f"test_sample_{next(self.sample_ids)}"
            self.addCleanup(sys.modules.pop, module, None)
            Path(tmp, f"{module}.py").write_text(textwrap.dedent(body))
            err = io.StringIO()
            with contextlib.redirect_stderr(err):
                code = timed_tests.main(["-s", tmp, *extra])
        return code, err.getvalue().replace(module, "test_sample")

    def test_limit_exceeded_exits_1_and_names_the_test(self):
        code, err = self.run_main("--max-test-s", "0.0001")
        self.assertEqual(code, 1)
        self.assertIn("test_sample.T.test_slow", err)
        self.assertIn("limit 0.0001 s", err)

    def test_no_limit_exits_0_and_prints_rows(self):
        code, err = self.run_main()
        self.assertEqual(code, 0)
        self.assertIn("test_sample.T.test_slow", err)

    def test_name_filter_is_or_over_substrings(self):
        code, err = self.run_main("-k", "test_slow", "-k", "test_fast")
        self.assertEqual(code, 0)
        self.assertIn("Ran 2 tests", err)
        _, err = self.run_main("-k", "test_slow")
        self.assertIn("Ran 1 test", err)


if __name__ == "__main__":
    unittest.main()
