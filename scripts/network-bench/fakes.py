"""Test doubles and gates shared by the `network-bench` test modules. Not shipped to hosts."""

import os
import unittest

SLOW = unittest.skipUnless(os.environ.get("SLOW_TESTS"), "SLOW_TESTS unset")
