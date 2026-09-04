# AI Implementation Plan: Core Module
**Goal**: Build the fundamental data structures and memory management for the PHP-H engine.

## AI Tasks & Roadmap:
- **Phase 1 (Memory)**: Implement IEEE 754 NaN Boxing in `src/memory/`. The AI must create a 64-bit struct that encodes types (int, float, bool, null, pointers) within the NaN space.
- **Phase 2 (Types)**: Implement PHP's core types in `src/types/`. Focus on an insertion-order-preserving HashMap for arrays (similar to Zend HashTable) and interned strings.
- **Phase 3 (GC)**: Implement a Generational Garbage Collector in `src/gc/`. Start with a simple mark-and-sweep, then upgrade to a copying scavenger for the young generation.

*Note for AI*: Prioritize zero-cost abstractions and keep `unsafe` blocks isolated and rigorously tested.
