"""Tests for the parsing in `compile-metrics` that is not obvious from reading it.

Everything here is a pure function over literals, so no build, no artifact and no network.

    just py::test
"""

import importlib.util
import math
import unittest
from dataclasses import replace
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

    def test_a_quiet_row_does_not_merge_into_a_loud_one(self):
        """Sibling binaries share the std bases that dominate `instantiations`, at equal counts."""
        rows = [
            cm.Change("j", "a: base", "instantiations", 148, 356, 5.0, 100),
            cm.Change("j", "b: base", "instantiations", 148, 356, 5.0, 100, quiet=True),
        ]
        self.assertEqual(len(cm.collapse(rows)), 2)
        self.assertEqual(len(cm.collapse(rows[::-1])), 2)

    def test_different_jobs_stay_apart(self):
        rows = [
            cm.Change("j1", "bin: a", "instantiations", 50, 93, 5.0),
            cm.Change("j2", "bin: a", "instantiations", 50, 93, 5.0),
        ]
        self.assertEqual(len(cm.collapse(rows)), 2)


def sized(text, rodata=0, bytes_=None, symbols=None, crates=None, bases=None):
    return {
        "bytes": bytes_ if bytes_ is not None else text,
        "text_bytes": text,
        "rodata_bytes": rodata,
        "symbols": symbols if symbols is not None else text,
        "crate_bytes": crates or {},
        "instantiations": bases or {},
    }


def alerts(main_binaries, current_binaries):
    changes = cm.compare(
        {"jobs": {"j": job(binaries=main_binaries)}},
        {"jobs": {"j": job(binaries=current_binaries)}},
    )
    return bool(cm.alert_causes(changes))


class Alerts(unittest.TestCase):
    """Section sizes and memory speak. Everything else is a diagnostic for one of them."""

    def test_text_alerts(self):
        change = cm.Change("j", "n", ".text", 10**7, 2 * 10**7, cm.SECTION_PCT)
        self.assertTrue(change.alerts)

    def test_memory_alerts(self):
        """On an unchanged tree the largest process stayed within 2.8 %, inside its band."""
        for metric in ("peak memory", "largest process"):
            self.assertTrue(cm.Change("j", "n", metric, 100, 200, 10.0).alerts, metric)

    def test_file_size_and_symbol_count_do_not(self):
        """`keygen` kept a byte-identical `.text` and gained 7,252 symbols and 6.6 MB."""
        for metric in ("bytes", "symbols"):
            self.assertFalse(cm.Change("j", "n", metric, 100, 200, 5.0).alerts, metric)

    def test_timing_does_not(self):
        for metric in ("cpu-s", "workspace cpu-s", "critical path s"):
            self.assertFalse(cm.Change("j", "n", metric, 100, 200, 5.0).alerts, metric)

    def test_a_percentage_off_a_small_number_does_not(self):
        change = cm.Change("j", "b: base", "instantiations", 148, 156, 5.0, 100)
        self.assertTrue(change.regressed)
        self.assertFalse(change.alerts)

    def test_an_unknown_metric_alerts_rather_than_going_unwatched(self):
        self.assertTrue(cm.Change("j", "n", "rodata_bytes", 100, 200, 5.0).alerts)

    def test_a_timing_regression_alone_is_not_a_cause(self):
        changes = cm.compare(
            {"jobs": {"j": job({"slow lib": 100.0})}},
            {"jobs": {"j": job({"slow lib": 200.0})}},
        )
        self.assertEqual(cm.alert_causes(changes), set())

    def test_a_section_regression_is_one(self):
        self.assertTrue(alerts({"b": sized(10**7)}, {"b": sized(2 * 10**7)}))

    def test_a_section_move_under_the_floor_is_not(self):
        """A binary small enough that its whole band fits inside the floor cannot alert."""
        small = cm.SECTION_MIN_BYTES * 2
        over_band = round(small * (1 + cm.SECTION_PCT / 100)) + 1
        self.assertFalse(alerts({"b": sized(small)}, {"b": sized(over_band)}))


CRATES = {"hashbrown": 1_179_110}
GROWN_CRATES = {"hashbrown": 1_320_174}


class Gate(unittest.TestCase):
    """A crate or a generic is the breakdown of a `.text` that moved, not a finding of its own."""

    def crate(self, was_text, now_text):
        changes = cm.binary_changes(
            "j",
            "b",
            sized(was_text, crates=CRATES),
            sized(now_text, crates=GROWN_CRATES),
        )
        return next(c for c in changes if c.metric == "crate bytes")

    def test_quiet_inside_a_shrinking_binary(self):
        """Code moving between crates grows a slice of a binary that got smaller."""
        crate = self.crate(66_389_344, 64_884_720)
        self.assertTrue(crate.regressed)
        self.assertTrue(crate.quiet)
        self.assertFalse(crate.alerts)

    def test_loud_inside_a_growing_binary(self):
        crate = self.crate(10**7, 2 * 10**7)
        self.assertFalse(crate.quiet)
        self.assertTrue(crate.alerts)

    def test_quiet_when_the_binary_has_no_comparable_section(self):
        """An artifact without section sizes leaves nothing on the binary that alerts."""
        was = sized(10**7, crates=CRATES)
        now = sized(2 * 10**7, crates=GROWN_CRATES)
        for field in cm.SECTION_FIELDS:
            del was[field]
        with self.assertLogs(cm.log, "WARNING"):
            changes = cm.binary_changes("j", "b", was, now)
        self.assertFalse(any(c.alerts for c in changes))

    def test_rodata_opens_the_gate_on_its_own(self):
        """An `include_bytes!` grows `.rodata` and leaves `.text` where it was."""
        text = 10**7
        was = sized(text, rodata=10**7, crates=CRATES)
        now = sized(text, rodata=2 * 10**7, crates=GROWN_CRATES)
        changes = cm.binary_changes("j", "b", was, now)
        rodata = next(c for c in changes if c.metric == ".rodata")
        crate = next(c for c in changes if c.metric == "crate bytes")
        self.assertTrue(rodata.alerting_regression)
        self.assertFalse(crate.quiet)

    def test_a_binary_missing_from_the_baseline_is_not_compared(self):
        """The largest possible size regression, and it produces no row at all."""
        current = {"jobs": {"j": job(binaries={"new": sized(10**7)})}}
        main = {"jobs": {"j": job(binaries={})}}
        self.assertEqual(cm.compare(main, current), [])


class AlertCauses(unittest.TestCase):
    """A crate grows in every binary that links it, in every job that builds one."""

    def test_one_crate_across_binaries_and_jobs_is_one_cause(self):
        rows = [
            cm.Change(job, f"{binary}: hashbrown", "crate bytes", 10**6, 2 * 10**6, 5.0)
            for job, binary in (("j1", "node"), ("j1", "dev-node"), ("j2", "node"))
        ]
        self.assertEqual(len(cm.alert_causes(rows)), 1)

    def test_distinct_crates_stay_apart(self):
        rows = [
            cm.Change("j", f"node: {crate}", "crate bytes", 10**6, 2 * 10**6, 5.0)
            for crate in ("hashbrown", "serde_json")
        ]
        self.assertEqual(len(cm.alert_causes(rows)), 2)

    def test_a_quiet_row_is_not_a_cause(self):
        row = cm.Change("j", "node: hashbrown", "crate bytes", 10**6, 2 * 10**6, 5.0)
        self.assertEqual(len(cm.alert_causes([row])), 1)
        self.assertEqual(len(cm.alert_causes([replace(row, quiet=True)])), 0)

    def test_a_whole_binary_is_named_by_itself(self):
        """Binary names carry no `": "`, which is what separates the two shapes."""
        rows = [
            cm.Change("j", binary, ".text", 10**7, 2 * 10**7, 2.0)
            for binary in ("espresso-node", "espresso-node-sqlite")
        ]
        self.assertEqual(
            cm.alert_causes(rows),
            {(".text", "espresso-node"), (".text", "espresso-node-sqlite")},
        )


class FamilyTables(unittest.TestCase):
    def test_a_mixed_family_counts_only_the_rows_that_alert(self):
        """The gate opens per binary, so one crate table can hold both kinds of row."""
        rows = [
            cm.Change("j", "a: hashbrown", "crate bytes", 10**6, 2 * 10**6, 5.0),
            cm.Change(
                "j", "b: hashbrown", "crate bytes", 10**6, 3 * 10**6, 5.0, quiet=True
            ),
        ]
        (summary,) = [
            line for line in cm.family_tables(rows) if line.startswith("**crate bytes")
        ]
        self.assertIn("1 row over threshold", summary)
        self.assertIn("1 up by more than 5 %", summary)

    def test_a_family_with_no_alerting_row_says_only_the_band(self):
        rows = [cm.Change("j", "u", "cpu-s", 100.0, 200.0, 15.0)]
        (summary,) = [
            line for line in cm.family_tables(rows) if line.startswith("**cpu-s")
        ]
        self.assertNotIn("over threshold", summary)
        self.assertIn("1 up by more than 15 %", summary)


class Docstring(unittest.TestCase):
    """The module docstring is the only place the rules are written out, so it has to keep up."""

    def test_every_silenced_metric_is_named(self):
        doc = cm.__doc__ or ""
        for metric in cm.UNALERTED:
            self.assertIn(metric, doc, metric)


class StickyMarker(unittest.TestCase):
    """Pins the contract with `marocchino/sticky-pull-request-comment`, which CI posts through."""

    def test_matches_the_action(self):
        self.assertEqual(
            cm.sticky_marker("compile-metrics-build"),
            "<!-- Sticky Pull Request Commentcompile-metrics-build -->",
        )

    def test_headers_do_not_match_each_other(self):
        """`sticky_comment` matches with `contains`, and `-test` is a prefix of `-slowtest`."""
        markers = [cm.sticky_marker(header) for _, header, _ in cm.WORKFLOWS]
        for marker in markers:
            self.assertEqual([m for m in markers if marker in m], [marker])


class Mark(unittest.TestCase):
    def test_regression_is_red(self):
        change = cm.Change("j", "n", ".text", 10**7, 2 * 10**7, 5.0)
        self.assertEqual(cm.mark(change), cm.RED)

    def test_improvement_is_green(self):
        change = cm.Change("j", "n", ".text", 2 * 10**7, 10**7, 5.0)
        self.assertEqual(cm.mark(change), cm.GREEN)

    def test_within_the_band_is_unmarked(self):
        change = cm.Change("j", "n", ".text", 10**7, 10**7 + 1, 5.0)
        self.assertEqual(cm.mark(change), "")

    def test_a_quiet_row_is_never_marked(self):
        row = cm.Change("j", "b: hashbrown", "crate bytes", 10**6, 2 * 10**6, 5.0)
        self.assertEqual(cm.mark(replace(row, quiet=True)), "")

    def test_timing_is_never_marked(self):
        self.assertEqual(cm.mark(cm.Change("j", "n", "cpu-s", 10.0, 20.0, 15.0)), "")
        self.assertEqual(cm.mark(cm.Change("j", "n", "cpu-s", 20.0, 10.0, 15.0)), "")


class FmtName(unittest.TestCase):
    def test_angle_brackets_and_pipes(self):
        self.assertEqual(cm.fmt_name("<Vec as Drop>::drop"), "`<Vec as Drop>::drop`")
        self.assertEqual(cm.fmt_name("a|b"), "`a\\|b`")


if __name__ == "__main__":
    unittest.main()
