#!/usr/bin/env python3
import os
import sys

script_dir = os.path.dirname(os.path.abspath(__file__))
suite = os.path.join(script_dir, "suites", "ecommerce_concurrency.py")
os.execv(sys.executable, [sys.executable, suite] + sys.argv[1:])
