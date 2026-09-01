"""Tests for the parsing in `llvm-lines` that is not obvious from reading it.

Everything here is a pure function over literals, so no build, no artifact and no network.

    just py::test
"""

import importlib.util
import json
import tempfile
import unittest
from importlib.machinery import SourceFileLoader
from pathlib import Path

SCRIPT = Path(__file__).with_name("llvm-lines")
_spec = importlib.util.spec_from_loader(
    "llvm_lines", SourceFileLoader("llvm_lines", str(SCRIPT))
)
assert _spec is not None
ll = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(ll)


TABLE_WITH_PERCENTS = """\
  Lines                 Copies               Function name
  -----                 ------               -------------
  1523423               38041                (TOTAL)
   772004 (50.7%, 50.7%)   8062 (21.2%, 71.9%)  core::ptr::drop_in_place
   112342 (7.4%, 58.1%)     11 (0.0%, 71.9%)  <futures_util::stream::FuturesUnordered<Fut> as futures_core::stream::Stream>::poll_next
"""

TABLE_BARE_NUMBERS = """\
  Lines                 Copies               Function name
  -----                 ------               -------------
  1523423               38041                (TOTAL)
   772004                 8062                core::ptr::drop_in_place
"""

# Verbatim rows from a real v0-mangled build: every crate path carries a `[<hex>]`
# disambiguator that changes with the toolchain and build settings.
TABLE_WITH_DISAMBIGUATORS = """\
  Lines                 Copies               Function name
  -----                 ------               -------------
  1000                  100                  (TOTAL)
   21 (42.0%, 42.0%)  1 (11.1%, 11.1%)  <std[1e3c4ec04c5261a9]::rt::lang_start<()>::{closure#0} as core[37f591cfbe66b0b1]::ops::function::FnOnce<()>>::call_once
    5 (10.0%, 52.0%)  1 (11.1%, 22.2%)  main
    2 (4.0%, 44.0%)   1 (11.1%, 33.3%)  espresso_node[4f5fb2aa37c475b]::main
"""


class ParseTable(unittest.TestCase):
    def test_total_extracted_and_excluded_from_rows(self):
        total_lines, total_copies, rows = ll.parse_table(TABLE_WITH_PERCENTS)
        self.assertEqual((total_lines, total_copies), (1523423, 38041))
        self.assertEqual(
            [name for name, _, _ in rows],
            [
                "core::ptr::drop_in_place",
                (
                    "<futures_util::stream::FuturesUnordered<Fut> as "
                    "futures_core::stream::Stream>::poll_next"
                ),
            ],
        )

    def test_row_values(self):
        _, _, rows = ll.parse_table(TABLE_WITH_PERCENTS)
        self.assertEqual(rows[0], ("core::ptr::drop_in_place", 772004, 8062))

    def test_bare_number_variant(self):
        total_lines, total_copies, rows = ll.parse_table(TABLE_BARE_NUMBERS)
        self.assertEqual((total_lines, total_copies), (1523423, 38041))
        self.assertEqual(rows, [("core::ptr::drop_in_place", 772004, 8062)])

    def test_empty_input_raises(self):
        with self.assertRaises(RuntimeError):
            ll.parse_table("")

    def test_no_total_row_raises(self):
        with self.assertRaises(RuntimeError):
            ll.parse_table(
                "  Lines                 Copies               Function name\n"
                "  -----                 ------               -------------\n"
                "   772004 (50.7%, 50.7%)   8062 (21.2%, 71.9%)  core::ptr::drop_in_place\n"
            )

    def test_v0_crate_disambiguators_stripped(self):
        _, _, rows = ll.parse_table(TABLE_WITH_DISAMBIGUATORS)
        self.assertEqual(
            [name for name, _, _ in rows],
            [
                (
                    "<std::rt::lang_start<()>::{closure#0} as "
                    "core::ops::function::FnOnce<()>>::call_once"
                ),
                "main",
                "espresso_node::main",
            ],
        )


class FunctionCrate(unittest.TestCase):
    def test_plain_path(self):
        self.assertEqual(
            ll.function_crate("espresso_types::v0::Header::new"), "espresso_types"
        )

    def test_impl_qualifier(self):
        self.assertEqual(
            ll.function_crate("<alloc::vec::Vec<u8> as core::ops::Drop>::drop"),
            "alloc",
        )

    def test_reference_self_type(self):
        self.assertEqual(
            ll.function_crate("<&mut alloc::vec::Vec<u8> as core::ops::Drop>::drop"),
            "alloc",
        )

    def test_c_symbol(self):
        self.assertEqual(ll.function_crate("memcpy"), "memcpy")

    def test_bare_type_parameter_falls_back_to_the_trait(self):
        """`T` has no crate path of its own; the blanket impl is the trait's."""
        self.assertEqual(
            ll.function_crate("<T as core::convert::Into<U>>::into"), "core"
        )

    def test_bare_future_type_parameter_falls_back_to_the_trait(self):
        self.assertEqual(
            ll.function_crate("<Fut as futures_core::future::Future>::poll"),
            "futures_core",
        )

    def test_arrow_in_self_type_does_not_break_qualifier_matching(self):
        """`->` is the one `>` that closes nothing; it must not end the qualifier early."""
        self.assertEqual(
            ll.function_crate("<fn(u8) -> u8 as core::ops::FnOnce<(u8,)>>::call_once"),
            "core",
        )

    def test_disambiguated_impl_qualifier(self):
        self.assertEqual(
            ll.function_crate(
                "<std::rt::lang_start<()>::{closure#0} as "
                "core::ops::function::FnOnce<()>>::call_once"
            ),
            "std",
        )

    def test_disambiguated_plain_path(self):
        self.assertEqual(ll.function_crate("espresso_node::main"), "espresso_node")

    def test_bare_main_has_no_path_so_the_whole_name_is_the_bucket(self):
        self.assertEqual(ll.function_crate("main"), "main")

    def test_disambiguated_trait_used_by_the_bare_type_parameter_fallback(self):
        self.assertEqual(
            ll.function_crate("<T as core::ops::function::FnOnce<()>>::call_once"),
            "core",
        )


class StripDisambiguators(unittest.TestCase):
    def test_hex_suffix_after_an_identifier_is_removed(self):
        self.assertEqual(
            ll.strip_disambiguators("std[1e3c4ec04c5261a9]::rt::lang_start"),
            "std::rt::lang_start",
        )

    def test_slice_type_is_not_a_disambiguator(self):
        self.assertEqual(
            ll.strip_disambiguators("<[u8] as some::Trait>::f"),
            "<[u8] as some::Trait>::f",
        )

    def test_array_type_is_not_a_disambiguator(self):
        self.assertEqual(ll.strip_disambiguators("[T; 4]"), "[T; 4]")


class CollectMetrics(unittest.TestCase):
    def test_crate_tally_and_totals(self):
        metrics = ll.collect_metrics("release", TABLE_WITH_PERCENTS)
        self.assertEqual(metrics["profile"], "release")
        self.assertEqual(metrics["total_lines"], 1523423)
        self.assertEqual(metrics["total_copies"], 38041)
        self.assertEqual(metrics["crates"]["core"], {"lines": 772004, "copies": 8062})
        self.assertEqual(
            metrics["crates"]["futures_util"], {"lines": 112342, "copies": 11}
        )
        self.assertIn(
            "<futures_util::stream::FuturesUnordered<Fut> as "
            "futures_core::stream::Stream>::poll_next",
            metrics["functions"],
        )

    def test_functions_capped_to_top_500(self):
        rows = "\n".join(f"    {n}   1  fn_{n}" for n in range(1, 600))
        text = f"  Lines Copies Function name\n  ----- ------ -------------\n  1000000 600 (TOTAL)\n{rows}\n"
        metrics = ll.collect_metrics("test", text)
        self.assertEqual(len(metrics["functions"]), 500)
        self.assertIn("fn_599", metrics["functions"])
        self.assertNotIn("fn_1", metrics["functions"])


class FmtName(unittest.TestCase):
    def test_pipe_escaped(self):
        self.assertEqual(ll.fmt_name("a|b"), "`a\\|b`")


class MissingWorkflowMessage(unittest.TestCase):
    def test_404_on_the_workflow_endpoint_gets_a_plain_explanation(self):
        error = (
            "`gh run list --workflow llvm-lines.yml` exited 1: HTTP 404: "
            "workflow llvm-lines.yml not found on the default branch (...)"
        )
        self.assertEqual(
            ll.missing_workflow_message(error, "llvm-lines.yml"),
            "llvm-lines.yml is not on the default branch yet, so gh cannot list its runs",
        )

    def test_other_errors_pass_through_unchanged(self):
        error = "`gh run list` exited 1: HTTP 500: internal error"
        self.assertEqual(ll.missing_workflow_message(error, "llvm-lines.yml"), error)


class MergeProfiles(unittest.TestCase):
    def test_merges_by_profile_field_not_file_name(self):
        with tempfile.TemporaryDirectory() as tmp:
            work = Path(tmp)
            release = {"profile": "release", "total_lines": 1, "total_copies": 1}
            test = {"profile": "test", "total_lines": 2, "total_copies": 2}
            (work / "llvm-lines-metrics-a.json").write_text(json.dumps(release))
            (work / "llvm-lines-metrics-b.json").write_text(json.dumps(test))
            merged = ll.merge_profiles(sorted(work.glob("*.json")))
        self.assertEqual(set(merged), {"release", "test"})
        self.assertEqual(merged["release"]["total_lines"], 1)


class Report(unittest.TestCase):
    def _metrics(self, total_lines, total_copies, crates, functions):
        return {
            "profile": "release",
            "total_lines": total_lines,
            "total_copies": total_copies,
            "crates": crates,
            "functions": functions,
        }

    def test_report_without_baseline_shows_absolute_numbers(self):
        current = {
            "profiles": {
                "release": self._metrics(
                    1000,
                    100,
                    {"core": {"lines": 600, "copies": 50}},
                    {"core::foo": {"lines": 600, "copies": 50}},
                )
            }
        }
        report = ll.report_markdown(current, None, "LLVM lines")
        self.assertIn(ll.NO_BASELINE, report)
        self.assertIn("| lines | 1,000 |", report)
        self.assertNotIn("delta", report)

    def test_report_with_baseline_shows_delta_and_missing_function(self):
        main_metrics = self._metrics(
            900,
            90,
            {"core": {"lines": 500, "copies": 40}},
            {"core::foo": {"lines": 500, "copies": 40}},
        )
        current_metrics = self._metrics(
            1000,
            100,
            {"core": {"lines": 600, "copies": 50}},
            {"core::bar": {"lines": 600, "copies": 50}},
        )
        current = {"profiles": {"release": current_metrics}}
        main = {"profiles": {"release": main_metrics}}
        report = ll.report_markdown(current, main, "LLVM lines")
        self.assertIn("| lines | 900 | 1,000 | +100 | +11.1% |", report)
        self.assertIn("core::foo", report)
        self.assertIn("core::bar", report)

    def test_function_movers_includes_one_sided_functions(self):
        main_metrics = self._metrics(
            1, 1, {}, {"only_main": {"lines": 50, "copies": 1}}
        )
        current_metrics = self._metrics(
            1, 1, {}, {"only_current": {"lines": 30, "copies": 1}}
        )
        rows = ll.function_movers(current_metrics, main_metrics)
        names = {name for name, _, _ in rows}
        self.assertEqual(names, {"only_main", "only_current"})

    def test_movers_table_when_nothing_moved(self):
        metrics = self._metrics(1, 1, {}, {"core::foo": {"lines": 50, "copies": 1}})
        self.assertEqual(
            ll.movers_table(metrics, metrics), ["No function's line count moved."]
        )

    def test_profile_missing_from_baseline_still_shows_absolute_numbers(self):
        current = {
            "profiles": {
                "release": self._metrics(
                    1000,
                    100,
                    {"core": {"lines": 600, "copies": 50}},
                    {"core::foo": {"lines": 600, "copies": 50}},
                )
            }
        }
        main = {"profiles": {}}
        report = ll.report_markdown(current, main, "LLVM lines")
        self.assertIn("No baseline for this profile on main.", report)
        self.assertIn("| lines | 1,000 |", report)
        self.assertNotIn("delta", report)


if __name__ == "__main__":
    unittest.main()
