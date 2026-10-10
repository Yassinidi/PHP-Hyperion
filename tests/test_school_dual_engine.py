#!/usr/bin/env python3
"""
Backward compatibility proxy for tests/integration/test_school_dual_engine.py
"""
import os
import sys

TARGET = os.path.join(os.path.dirname(os.path.abspath(__file__)), "integration", "test_school_dual_engine.py")
os.execv(sys.executable, [sys.executable, TARGET] + sys.argv[1:])
