#!/usr/bin/env python3
"""
Backward compatibility proxy for tests/performance/verify_memory_rss.py
"""
import os
import sys

TARGET = os.path.join(os.path.dirname(os.path.abspath(__file__)), "performance", "verify_memory_rss.py")
os.execv(sys.executable, [sys.executable, TARGET] + sys.argv[1:])
