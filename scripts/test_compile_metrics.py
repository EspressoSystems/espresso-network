"""Tests for the parsing in `compile-metrics` that is not obvious from reading it.

Everything here is a pure function over literals, so no build, no artifact and no network.

    just py::test
"""

import importlib.util
import math
import unittest
from importlib.machinery import SourceFileLoader
from pathlib import Path

SCRIPT = Path(__file__).with_name("compile-metrics")
_spec = importlib.util.spec_from_loader(
    "compile_metrics", SourceFileLoader("compile_metrics", str(SCRIPT))
)
assert _spec is not None
cm = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(cm)


def unit(index, name, duration, unblocked=(), rmeta=(), frontend=None):
    return {
        "i": index,
        "name": name,
        "target": "",
        "duration": duration,
        "unblocked_units": list(unblocked),
        "unblocked_rmeta_units": list(rmeta),
        "sections": [["frontend", {"start": 0.0, "end": frontend}]]
        if frontend
        else None,
    }


class SplitQualifier(unittest.TestCase):
    def test_leading_impl_qualifier(self):
        self.assertEqual(
            cm.split_qualifier("<Vec<u8> as Drop>::drop"),
            ("<Vec<u8> as Drop>", "::drop"),
        )

    def test_argument_list_is_not_a_qualifier(self):
        self.assertEqual(cm.split_qualifier("foo::<u8>"), ("", "foo::<u8>"))

    def test_unbalanced(self):
        self.assertEqual(cm.split_qualifier("<Vec<u8>"), ("", "<Vec<u8>"))


class StripGenericArgs(unittest.TestCase):
    def test_nested_arguments(self):
        self.assertEqual(
            cm.strip_generic_args("Map<Filter<I, P>, F>::next"), "Map::next"
        )

    def test_keeps_impl_qualifier(self):
        self.assertEqual(cm.strip_generic_args("<Vec<u8> as Drop>"), "<Vec as Drop>")


class GenericBase(unittest.TestCase):
    def test_v0_arguments_stripped(self):
        self.assertEqual(
            cm.generic_base("core::ptr::drop_in_place::<espresso_types::Header>"),
            "core::ptr::drop_in_place",
        )

    def test_impl_qualifier_kept(self):
        self.assertEqual(
            cm.generic_base("<alloc::vec::Vec<u8> as core::ops::Drop>::drop"),
            "<alloc::vec::Vec as core::ops::Drop>::drop",
        )

    def test_legacy_hash_stripped(self):
        self.assertEqual(
            cm.generic_base("std::rt::lang_start::h0123456789abcdef"),
            "std::rt::lang_start",
        )

    def test_function_pointer_argument(self):
        """`->` is the one `>` that closes nothing; these were dropped entirely once."""
        self.assertEqual(cm.generic_base("foo::bar::<fn(u8) -> u8>"), "foo::bar")
        self.assertEqual(
            cm.generic_base("<F as FnOnce<(A,)>>::call_once::<fn(u8) -> u8>"),
            "<F as FnOnce>::call_once",
        )

    def test_arrow_outside_an_argument_list_survives(self):
        self.assertIsNone(cm.generic_base("a -> b"))

    def test_not_an_instantiation(self):
        self.assertIsNone(cm.generic_base("memcpy"))
        self.assertIsNone(cm.generic_base("<Foo as Bar>::baz"))


class SymbolCrate(unittest.TestCase):
    def test_impl_qualifier(self):
        self.assertEqual(
            cm.symbol_crate("<alloc::vec::Vec<u8> as Drop>::drop"), "alloc"
        )

    def test_plain_path(self):
        self.assertEqual(
            cm.symbol_crate("espresso_types::v0::Header::new"), "espresso_types"
        )

    def test_c_symbol(self):
        self.assertEqual(cm.symbol_crate("memcpy"), "memcpy")


class CriticalPath(unittest.TestCase):
    def test_longest_chain_wins(self):
        units = [
            unit(0, "root", 1.0, unblocked=[1, 2]),
            unit(1, "short", 2.0),
            unit(2, "long", 5.0),
        ]
        self.assertEqual(
            cm.critical_path(units),
            [("root lib", 1.0), ("long lib", 5.0)],
        )

    def test_rmeta_edge_waits_on_the_frontend_only(self):
        """A dependent needing only rmeta does not wait out its dependency's codegen."""
        units = [
            unit(0, "dep", 10.0, rmeta=[1], frontend=3.0),
            unit(1, "user", 9.0),
        ]
        self.assertEqual(cm.critical_path(units), [("dep lib", 3.0), ("user lib", 9.0)])

    def test_own_duration_beats_a_shorter_successor_chain(self):
        """A codegen tail nothing waits on still has to finish before the build does."""
        units = [
            unit(0, "dep", 10.0, rmeta=[1], frontend=1.0),
            unit(1, "user", 2.0),
        ]
        self.assertEqual(cm.critical_path(units), [("dep lib", 10.0)])

    def test_edge_to_a_unit_outside_the_report_is_dropped(self):
        units = [unit(0, "root", 1.0, unblocked=[99])]
        self.assertEqual(cm.critical_path(units), [("root lib", 1.0)])

    def test_empty(self):
        self.assertEqual(cm.critical_path([]), [])


def job(units=None, binaries=None):
    return {"units": units or {}, "binaries": binaries or {}}


def compare(main_units, current_units):
    return cm.compare(
        {"jobs": {"j": job(main_units)}}, {"jobs": {"j": job(current_units)}}
    )


class CompareUnits(unittest.TestCase):
    """A unit that stops being built is the whole point of a compile-time change."""

    def test_unit_gone_from_this_run(self):
        (change,) = compare({"dropped lib": 40.0}, {})
        self.assertEqual((change.main, change.current), (40.0, 0.0))
        self.assertEqual(change.delta_pct, -100.0)

    def test_unit_new_in_this_run(self):
        (change,) = compare({}, {"added lib": 40.0})
        self.assertEqual((change.main, change.current), (0.0, 40.0))
        self.assertTrue(change.regressed)

    def test_short_unit_stays_out_either_way(self):
        self.assertEqual(compare({"tiny lib": 1.0}, {}), [])
        self.assertEqual(compare({}, {"tiny lib": 1.0}), [])

    def test_unit_in_both_is_compared(self):
        (change,) = compare({"kept lib": 40.0}, {"kept lib": 60.0})
        self.assertEqual((change.main, change.current), (40.0, 60.0))


class DeltaPct(unittest.TestCase):
    def test_growth_from_nothing_is_unbounded(self):
        self.assertEqual(
            cm.Change("j", "n", "cpu-s", 0.0, 5.0, 15.0).delta_pct, math.inf
        )

    def test_nothing_to_nothing_is_flat(self):
        self.assertEqual(cm.Change("j", "n", "cpu-s", 0.0, 0.0, 15.0).delta_pct, 0.0)


class Collapse(unittest.TestCase):
    """Four vtable slots of one `dyn Error` impl always move together."""

    def test_identical_deltas_merge(self):
        rows = [
            cm.Change("j", f"bin: base{i}", "instantiations", 50, 93, 5.0)
            for i in range(4)
        ]
        (merged,) = cm.collapse(rows)
        self.assertEqual(merged.count, 4)
        self.assertEqual(merged.change.name, "bin: base0")

    def test_different_values_stay_apart(self):
        rows = [
            cm.Change("j", "bin: a", "instantiations", 50, 93, 5.0),
            cm.Change("j", "bin: b", "instantiations", 50, 94, 5.0),
        ]
        self.assertEqual(len(cm.collapse(rows)), 2)

    def test_different_jobs_stay_apart(self):
        rows = [
            cm.Change("j1", "bin: a", "instantiations", 50, 93, 5.0),
            cm.Change("j2", "bin: a", "instantiations", 50, 93, 5.0),
        ]
        self.assertEqual(len(cm.collapse(rows)), 2)


class Alerts(unittest.TestCase):
    """Timing is a measurement; binary size is a property of the artifact."""

    def test_binary_metrics_alert(self):
        for metric in ("bytes", "text_bytes", "symbols", "instantiations"):
            self.assertTrue(cm.Change("j", "n", metric, 100, 200, 5.0).alerts, metric)

    def test_timing_and_memory_do_not(self):
        for metric in ("cpu-s", "workspace cpu-s", "critical path s", "peak memory"):
            self.assertFalse(cm.Change("j", "n", metric, 100, 200, 5.0).alerts, metric)

    def test_a_timing_regression_alone_posts_no_comment(self):
        current = {"jobs": {"j": job({"slow lib": 200.0})}}
        main = {"jobs": {"j": job({"slow lib": 100.0})}}
        _, regressed = cm.report_markdown(current, main, "t")
        self.assertFalse(regressed)

    def test_a_binary_regression_posts_one(self):
        def binary(size):
            return {"b": {"bytes": size, "text_bytes": size, "symbols": size}}

        current = {"jobs": {"j": job(binaries=binary(200))}}
        main = {"jobs": {"j": job(binaries=binary(100))}}
        _, regressed = cm.report_markdown(current, main, "t")
        self.assertTrue(regressed)


class Mark(unittest.TestCase):
    def test_regression_is_red(self):
        self.assertEqual(cm.mark(cm.Change("j", "n", "bytes", 10.0, 20.0, 5.0)), cm.RED)

    def test_improvement_is_green(self):
        self.assertEqual(
            cm.mark(cm.Change("j", "n", "bytes", 20.0, 10.0, 5.0)), cm.GREEN
        )

    def test_within_the_band_is_unmarked(self):
        self.assertEqual(cm.mark(cm.Change("j", "n", "bytes", 10.0, 10.1, 5.0)), "")

    def test_timing_is_never_marked(self):
        self.assertEqual(cm.mark(cm.Change("j", "n", "cpu-s", 10.0, 20.0, 15.0)), "")
        self.assertEqual(cm.mark(cm.Change("j", "n", "cpu-s", 20.0, 10.0, 15.0)), "")


class FmtName(unittest.TestCase):
    def test_angle_brackets_and_pipes(self):
        self.assertEqual(cm.fmt_name("<Vec as Drop>::drop"), "`<Vec as Drop>::drop`")
        self.assertEqual(cm.fmt_name("a|b"), "`a\\|b`")


if __name__ == "__main__":
    unittest.main()
