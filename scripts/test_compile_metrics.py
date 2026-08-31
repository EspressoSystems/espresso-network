"""Tests for the parsing in `compile-metrics` that is not obvious from reading it.

Everything here is a pure function over literals, so no build, no artifact and no network.

    just py::test
"""

import importlib.util
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


class FmtName(unittest.TestCase):
    def test_angle_brackets_and_pipes(self):
        self.assertEqual(cm.fmt_name("<Vec as Drop>::drop"), "`<Vec as Drop>::drop`")
        self.assertEqual(cm.fmt_name("a|b"), "`a\\|b`")


if __name__ == "__main__":
    unittest.main()
