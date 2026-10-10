#!/usr/bin/env python3
"""
Backward compatibility proxy for tests/security/security_and_compatibility_audit.py
"""
import os
import sys

TARGET = os.path.join(os.path.dirname(os.path.abspath(__file__)), "security", "security_and_compatibility_audit.py")
os.execv(sys.executable, [sys.executable, TARGET] + sys.argv[1:])
