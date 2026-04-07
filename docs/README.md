# Documentation

Welcome to the Blaniel NPC API documentation. This directory contains in-depth guides for developers who want to dive deeper into the system's architecture and implementation.

## Documentation Files

### [OPTIMIZATIONS.md](OPTIMIZATIONS.md)
**Real-Time Performance Optimizations**

Overview of all optimizations implemented to achieve sub-20ms latency for NPC interactions with 150+ simultaneous NPCs.

**Topics covered:**
- Multi-Key Load Balancer
- Predictive Cache Warming
- Hybrid Response System
- Priority Queue System
- Configuration by scale (10-200+ NPCs)
- Monitoring and troubleshooting

**For:** Users wanting to understand the real-time capabilities and optimize their deployment.

---

### [PERFORMANCE.md](PERFORMANCE.md)
**Performance Benchmarks and Tuning**

Technical guide with critical fixes, benchmarks, and performance tuning for achieving sub-20ms response times.

**Topics covered:**
- Critical fixes implemented
- AI provider integration (Venice, Together)
- Running benchmarks
- Monitoring with tracing and metrics
- Optimization tips (cache warming, connection pooling, etc.)
- Troubleshooting common performance issues
- Performance checklist for production

**For:** Developers needing detailed performance analysis, benchmarking, and tuning guidance.

---

### [ACTION-SYSTEM.md](ACTION-SYSTEM.md)
**AI Action System - Complete Guide**

Comprehensive documentation of the AI action system with all implemented optimizations and new features.

**Topics covered:**
- 20 improvements (8 optimizations, 12 new features)
- Action Priority System
- Action History and Analytics
- Conditional Actions
- Action Templates
- Action Queue System
- Action Cooldown Management
- Action Feedback Loop
- Action Interruption System
- Action Dependencies
- Composite Actions (Multi-Step)
- API endpoints and examples

**For:** Developers implementing or extending the action system, or wanting deep understanding of NPC behavior.

---

## Quick Start

1. **Read the [main README](../README.md)** - Overview, quick start, and basic usage
2. **For performance needs:** Check [OPTIMIZATIONS.md](OPTIMIZATIONS.md)
3. **For benchmarking/tuning:** Check [PERFORMANCE.md](PERFORMANCE.md)
4. **For action system details:** Check [ACTION-SYSTEM.md](ACTION-SYSTEM.md)

## API Documentation

For API endpoint details, see the main README.md under "API Reference" section.

To generate Rust documentation:
```bash
cargo doc --open
```

## Binding-Specific Documentation

- **C# (Unity):** See [bindings/csharp/README.md](../bindings/csharp/README.md)
- **Python:** See [bindings/python/README.md](../bindings/python/README.md)

## Contributing

When adding new features or fixes to documentation:
1. Use clear, concise English
2. Include code examples where applicable
3. Cross-reference related sections
4. Update this index if adding new documentation files

## Support

For issues or questions:
1. Check the relevant documentation file
2. Review the main README.md
3. Open an issue on GitHub

---

**Last updated:** April 3, 2026
