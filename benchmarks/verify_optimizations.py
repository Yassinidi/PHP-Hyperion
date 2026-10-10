#!/usr/bin/env python3
"""
Backward compatibility proxy for benchmarks/stress/verify_optimizations.py
"""
import os
import sys

TARGET = os.path.join(os.path.dirname(os.path.abspath(__file__)), "stress", "verify_optimizations.py")
os.execv(sys.executable, [sys.executable, TARGET] + sys.argv[1:])
