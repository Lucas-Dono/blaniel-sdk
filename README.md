# Blaniel NPC SDK

High-performance Rust SDK for AI-powered NPCs in games with Python, C#, C FFI, and Unity bindings.

## Features

- **Multi-language bindings**: Python, C#, C, Unity
- **Emotional intelligence**: NPCs with realistic emotional responses
- **Vector memory**: Long-term memory using embeddings
- **Proactive behavior**: NPCs that initiate conversations naturally
- **High performance**: Built in Rust for zero-cost abstractions

## Quick Start

```rust
use blaniel_npc::NPC;

// Create an NPC
let npc = NPC::new("Luna", "A friendly companion");

// Interact with the NPC
let response = npc.chat("Hello!").await?;
```

## Documentation

See the [main Blaniel documentation](https://github.com/Lucas-Dono/blaniel) for detailed guides.

## License

MIT License - see [LICENSE](LICENSE) for details.
