# Blaniel NPC API

Open-source Rust microservice for integrating AI-powered NPCs into any game. Provides a high-performance REST API with emotions, pathfinding, dialogue, and configurable actions — designed to work with the [Blaniel](https://github.com/Lucas-Dono/blaniel) platform or any OpenAI-compatible backend.

> **Note:** This is part of [Blaniel](https://github.com/Lucas-Dono/blaniel), a one-person open-source project. If you find it useful, consider [supporting its development](https://tecito.app/blaniel).

## Features

- **Emotional NPC Chat** — AI responses carry emotion tags, animation hints, and source routing (cache/LLM/template)
- **A\* Pathfinding** — 3D pathfinding with diagonal movement, water/lava avoidance, and world bounds
- **Navigation System** — 4 modes: `word` (named locations), `coordinate`, `hybrid`, `free`
- **Action System** — 18 game genres with 19 action categories, fully configurable per-NPC
- **LLM Provider Registry** — 14 presets (OpenAI, Groq, Anthropic, Gemini, etc.) with automatic failover
- **Multi-level Cache** — LRU memory + Redis with smart TTL strategies, sub-millisecond cached reads
- **CLI + SDK** — `blaniel` command-line tool, Rust SDK, C FFI, Python bindings, C# (Unity) client
- **Game Engine Support** — Unity, Unreal Engine, Godot, Pygame, and any C/C++ engine

## Architecture

```
Game Engine                    Blaniel NPC API              External
─────────────                  ─────────────────              ────────

Unity (C#)     ──┐                                    ┌──> OpenAI
Unreal (C/FFI) ──┤    ┌──────────────────────┐        ├──> Groq
Godot (C/FFI)  ──┼───>│   Rust API (Axum)    │        ├──> Anthropic
Pygame (PyO3)  ──┤    │                      │        ├──> Gemini
CLI            ──┤    │  ┌─────┐  ┌───────┐  │        └──> 10+ more
Any HTTP client ──┘    │  │Cache│  │ LLM   │  │
                       │  │ LRU │  │Client  │  │        ┌──> PostgreSQL
                       │  │Redis│  │Retry   │  │───────>├──> Redis
                       │  └─────┘  └───────┘  │        └──> Next.js Backend
                       │                      │
                       │  Pathfinding Engine   │
                       │  Navigation Engine    │
                       │  Action System        │
                       │  Dialogue Manager     │
                       └──────────────────────┘
```

## Quick Start

### Prerequisites

- Rust 1.75+ (`rustup update stable`)
- PostgreSQL 16+
- Redis 7+

### Docker (Recommended)

```bash
cp .env.example .env
# Edit .env with your credentials
docker-compose up -d
```

### Local Development

```bash
cp .env.example .env
# Edit .env

# Start dependencies
docker-compose up -d postgres redis

# Build and run
cargo run --release -p npc-api
```

### Verify

```bash
curl http://localhost:3001/health
# {"status":"ok","timestamp":...,"version":"0.1.0"}
```

## Workspace Structure

```
rust-npc-api/
├── crates/
│   ├── api/            HTTP server (Axum routes, handlers, middleware)
│   ├── core/           Business logic (LLM client, pathfinding, navigation, dialogue)
│   ├── db/             PostgreSQL queries (read-only, SQLx)
│   ├── cache/          Multi-level cache (LRU memory + Redis + TTL strategies)
│   ├── types/          Shared types (60+ models, enums, configs)
│   ├── sdk/            BlanielClient — Rust HTTP client library
│   ├── cli/            `blaniel` command-line tool
│   └── ffi/            C FFI bindings (cdylib/staticlib)
├── bindings/
│   ├── python/         Python bindings (PyO3)
│   └── csharp/         Unity/.NET C# client
├── sdk/unity/          Legacy Unity SDK (pre-SDK crate)
├── tests/              Integration tests
├── benches/            Benchmarks (criterion)
├── scripts/            Setup & migration scripts
├── Dockerfile          Multi-stage Docker build
└── docker-compose.yml  PostgreSQL + Redis + API
```

## API Reference

### Public Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `GET` | `/metrics` | Prometheus metrics |

### NPC Management (JWT Auth)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/npc/:id` | Get NPC state |
| `GET` | `/api/v1/npc/nearby` | Find NPCs in radius |
| `POST` | `/api/v1/npc/batch-state` | Batch state retrieval |
| `POST` | `/api/v1/npc/:id/move` | Move NPC to position |
| `POST` | `/api/v1/npc/:id/chat` | Chat with NPC |
| `POST` | `/api/v1/npc/:id/pathfind` | Calculate A\* path |

### Navigation (JWT Auth)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/npc/:id/navigate` | Navigate to target (word/coordinate/relative) |
| `GET` | `/api/v1/npc/:id/navigation-config` | Get navigation config |
| `POST` | `/api/v1/npc/:id/navigation-config` | Set navigation config |
| `GET` | `/api/v1/navigation/scene/:scene_id/positions` | List scene positions |
| `POST` | `/api/v1/navigation/scene/register` | Register scene with named positions |

### Action System (JWT Auth)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/actions/catalog` | All game genres and default actions |
| `GET` | `/api/v1/actions/catalog/:genre` | Actions for a specific genre |
| `GET` | `/api/v1/npc/:id/action-config` | Get NPC action configuration |
| `POST` | `/api/v1/npc/:id/action-config` | Set NPC action configuration |
| `GET` | `/api/v1/npc/:id/ai-context` | Get generated AI context prompt |
| `POST` | `/api/v1/npc/:id/action-plan` | Create autonomous action plan |

### LLM Providers (JWT Auth)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/llm/presets` | List 14 provider presets |
| `GET` | `/api/v1/llm/providers` | List configured providers |
| `POST` | `/api/v1/llm/providers` | Register a new provider |
| `GET` | `/api/v1/llm/providers/:name` | Get provider status |
| `POST` | `/api/v1/llm/providers/:name` | Unregister provider |
| `GET` | `/api/v1/llm/metrics/global` | Global LLM metrics |
| `GET` | `/api/v1/llm/metrics/agent/:id` | Agent-specific metrics |
| `GET` | `/api/v1/llm/metrics/user/:id` | User-specific metrics |
| `GET` | `/api/v1/llm/metrics/top-agents` | Top agents by usage |
| `GET` | `/api/v1/llm/metrics/recent` | Recent LLM requests |

### Other (JWT Auth)

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/dialogue/ambient` | Generate ambient NPC dialogue |
| `POST` | `/api/v1/webhook/invalidate` | Cache invalidation (HMAC-SHA256 or JWT) |

### Example Requests

```bash
# Chat with NPC
curl -X POST \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"message": "What do you sell?", "context": {"position": {"x": 100, "y": 64, "z": 200, "world": "overworld"}, "nearby_players": ["player1"]}}' \
  http://localhost:3001/api/v1/npc/abc-123/chat

# Navigate NPC to named location
curl -X POST \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"target": {"type": "word", "name": "tavern"}}' \
  http://localhost:3001/api/v1/npc/abc-123/navigate

# Register LLM provider
curl -X POST \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"provider_type": "groq", "api_key": "gsk_your_key", "model": "llama-3.3-70b-versatile"}' \
  http://localhost:3001/api/v1/llm/providers

# Set action config for RPG NPC
curl -X POST \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "game_genre": "rpg",
    "enabled_actions": ["combat", "trading", "social", "exploration"],
    "introduction": "You are a merchant in the village square.",
    "action_explain": "You can trade goods and chat with travelers.",
    "movement_explain": "You walk around the market area."
  }' \
  http://localhost:3001/api/v1/npc/abc-123/action-config
```

## SDK Usage

### Rust Library

```toml
# Cargo.toml
[dependencies]
blaniel-sdk = { path = "crates/sdk" }
```

```rust
use blaniel_sdk::{BlanielClient, BlanielConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = BlanielClient::connect("http://localhost:3001", "jwt-token")?;

    // Chat
    let response = client.chat_simple("npc-123", "Hello!").await?;
    println!("{}: {} (emotion: {})", "NPC", response.response, response.emotion);

    // Navigate to named location
    let nav = client.navigate_to_word("npc-123", "tavern").await?;
    println!("Navigating {} blocks", nav.distance.unwrap_or(0.0));

    // Pathfinding
    let path = client.pathfind(
        "npc-123",
        Position3D::new(0.0, 64.0, 0.0, "overworld".into()),
        Position3D::new(100.0, 64.0, 50.0, "overworld".into()),
        None,
    ).await?;
    println!("Path: {} steps in {}ms", path.length(), path.computed_in_ms);

    // Quick setup: configure NPC for RPG in one call
    let result = client.setup_npc(
        "npc-123",
        GameGenre::Rpg,
        NavigationMode::Hybrid,
        Some("village"),
        vec![
            ScenePosition {
                name: "tavern".into(),
                position: Position3D::new(100.0, 64.0, 200.0, "overworld".into()),
                aliases: vec!["bar".into(), "inn".into()],
                tags: vec!["building".into(), "social".into()],
            },
        ],
    ).await?;

    Ok(())
}
```

### CLI

```bash
# Install
cargo install --path crates/cli

# Initialize (saves config to ~/.blaniel/config.json)
blaniel init --url https://api.blaniel.com --key YOUR_JWT

# NPC operations
blaniel npc get abc-123
blaniel npc chat abc-123 "Hello, how are you?"
blaniel npc move abc-123 --x 100 --y 64 --z 200 --world overworld
blaniel npc pathfind abc-123 --from_x 0 --from_y 64 --from_z 0 --to_x 100 --to_y 64 --to_z 50
blaniel npc navigate abc-123 --name tavern
blaniel npc setup abc-123 --genre rpg --nav-mode hybrid

# LLM providers
blaniel llm presets
blaniel llm add --provider groq --key gsk_your_key
blaniel llm add --provider openai --key sk_your_key --model gpt-4o-mini
blaniel llm list
blaniel llm metrics --global

# Scenes
blaniel scene register --scene-id village --world overworld --file positions.json
blaniel scene list village

# Actions
blaniel actions catalog
blaniel actions get abc-123
blaniel actions set abc-123 --genre rpg --introduction "You are a merchant."

# Interactive setup wizard
blaniel setup abc-123
```

### Python (Pygame / Python games)

```bash
# Build
cd bindings/python && maturin develop --release
```

```python
from blaniel import Blaniel

client = Blaniel("http://localhost:3001", "jwt-token", timeout_ms=5000)

# Chat with NPC
response = client.chat("npc-123", "What's happening today?")
print(response["response"])      # NPC's reply
print(response["emotion"])       # "joy", "anger", etc.
print(response["animation"])     # "wave", "nod", etc.

# Navigate
result = client.navigate_to("npc-123", "tavern")
print(f"Distance: {result['distance']} blocks")

# Configure actions
client.set_action_config("npc-123", """
{
    "game_genre": "rpg",
    "enabled_actions": ["combat", "trading", "social"],
    "introduction": "You are a blacksmith.",
    "action_explain": "You craft weapons and sell them.",
    "movement_explain": "You stay near your forge."
}
""")

# Register LLM provider
client.register_llm_provider("groq", "gsk_your_key")
```

### C# (Unity / .NET)

Copy `bindings/csharp/BlanielClient.cs` into your Unity project's `Assets/Scripts/` folder. No native dependencies.

```csharp
using Blaniel.NPC;

public class NPCManager : MonoBehaviour
{
    private BlanielClient _client;

    void Start()
    {
        _client = new BlanielClient("https://api.blaniel.com", "jwt-token");
    }

    public async void TalkToNPC(string npcId, string message)
    {
        var response = await _client.Chat(npcId, message);
        Debug.Log($"NPC: {response.Response} (emotion: {response.Emotion})");
    }

    public async void SetupScene()
    {
        await _client.RegisterScene("village", "overworld", new[]
        {
            new ScenePosition
            {
                Name = "tavern",
                Position = new Position(100, 64, 200),
                Aliases = new List<string> { "bar", "inn" }
            }
        });

        await _client.SetNavigationConfig("npc-123", new NavigationConfig
        {
            Mode = "hybrid",
            SceneId = "village"
        });
    }
}
```

### C FFI (Unreal Engine / Godot / C++)

Build the shared library:

```bash
cargo build --release -p blaniel-ffi
# Output: target/release/libblaniel_ffi.a (static) + .so (shared)
```

```c
#include "blaniel.h"

int main() {
    // Initialize
    blaniel_init("https://api.blaniel.com", "jwt-token", 30000);

    // Chat
    char* response = blaniel_chat("npc-123", "Hello!");
    printf("Response: %s\n", response);
    blaniel_free_string(response);

    // Navigate
    char* nav = blaniel_navigate_to("npc-123", "tavern");
    blaniel_free_string(nav);

    // Cleanup
    blaniel_cleanup();
    return 0;
}
```

## Navigation System

### Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| `word` | Navigate to named locations (`[navigate:tavern]`) | Dialogue-driven, adventure games |
| `coordinate` | Navigate to exact XYZ positions | Strategy, sandbox games |
| `hybrid` | Supports both word and coordinate | Most games (default) |
| `free` | No navigation constraints | Open world, free roam |

### Scene Registration

```json
{
  "scene_id": "village",
  "world": "overworld",
  "positions": [
    {
      "name": "tavern",
      "position": {"x": 100, "y": 64, "z": 200, "world": "overworld"},
      "aliases": ["bar", "inn", "pub"],
      "tags": ["building", "social"]
    },
    {
      "name": "forge",
      "position": {"x": 120, "y": 64, "z": 180, "world": "overworld"},
      "aliases": ["blacksmith"],
      "tags": ["building", "crafting"]
    }
  ]
}
```

### Movement Restrictions

```json
{
  "mode": "hybrid",
  "restrictions": {
    "max_distance": 100.0,
    "bounds": {
      "min_x": -500, "min_y": 0, "min_z": -500,
      "max_x": 500, "max_y": 128, "max_z": 500,
      "world": "overworld"
    },
    "blocked_positions": [
      {"x": 50, "y": 64, "z": 50, "world": "overworld"}
    ],
    "allowed_worlds": ["overworld"]
  },
  "scene_id": "village",
  "default_speed": 4.0
}
```

## Action System

### Game Genres (18 built-in)

`rpg` `fighting` `magic` `adventure` `romance` `simulation` `visual_novel` `survival` `horror` `sandbox` `strategy` `sports` `puzzle` `platformer` `shooter` `stealth` `racing` `rhythm` + any custom string

### Action Categories (19)

| Category | Description |
|----------|-------------|
| `combat` | Attack, defend, dodge, use weapons |
| `magic` | Cast spells, control elements |
| `crafting` | Create items, repair equipment |
| `trading` | Buy, sell, barter, manage inventory |
| `social` | Talk, befriend, persuade |
| `exploration` | Discover locations, map terrain |
| `stealth` | Sneak, hide, infiltrate |
| `farming` | Plant, harvest, tend crops |
| `fishing` | Fish, collect aquatic resources |
| `mining` | Extract minerals, dig tunnels |
| `building` | Construct structures, place blocks |
| `cooking` | Prepare food, mix ingredients |
| `healing` | Heal wounds, apply buffs |
| `music` | Play instruments, sing |
| `emotes` | Express emotions, gesture |
| `navigation` | Move, travel between locations |
| `interaction` | Use objects, activate mechanisms |
| `dialogue` | Complex dialogue trees |
| `quest` | Give, accept, complete quests |

### NPC Configuration

```json
{
  "game_genre": "rpg",
  "enabled_actions": ["combat", "trading", "social"],
  "custom_actions": [
    {
      "name": "brew_potion",
      "description": "Brews a healing potion",
      "animation": "craft",
      "cooldown_ms": 5000,
      "target_required": false,
      "parameters": {"ingredient": "herb"}
    }
  ],
  "introduction": "You are an alchemist in the village.",
  "action_explain": "You brew potions and sell them to adventurers.",
  "movement_explain": "You stay in your shop but can walk to the market."
}
```

The `introduction`, `action_explain`, and `movement_explain` fields are mandatory. They build the AI context that tells the NPC who it is, what it can do, and how it can move.

## LLM Provider System

### Supported Providers (14 presets)

| Provider | Default Model | Streaming | Functions |
|----------|---------------|-----------|-----------|
| OpenAI | gpt-4o-mini | Yes | Yes |
| Anthropic | claude-sonnet-4-20250514 | Yes | Yes |
| Gemini | gemini-2.0-flash | Yes | Yes |
| xAI | grok-3-mini-fast | Yes | Yes |
| Groq | llama-3.3-70b-versatile | Yes | Yes |
| DeepSeek | deepseek-chat | Yes | No |
| Mistral | mistral-small-latest | Yes | Yes |
| Together | meta-llama/Llama-3.3-70B-Instruct-Turbo | Yes | No |
| Venice | venice-uncensored-role-play | Yes | No |
| Fireworks | llama-v3-8b-instruct | Yes | No |
| OpenRouter | meta-llama/llama-3.3-70b-instruct | Yes | Yes |
| Perplexity | sonar | Yes | No |
| Ollama | llama3 (local) | Yes | No |
| LM Studio | default (local) | Yes | No |

### Failover Strategies

The registry supports 4 strategies when multiple providers are configured:

- **Priority** — Uses providers in order of priority, falls back on error
- **RoundRobin** — Distributes requests evenly across providers
- **LeastLatency** — Routes to the provider with lowest average latency
- **Random** — Random selection (for testing)

### Circuit Breaker

Each provider tracks:
- Consecutive errors (opens circuit after 5)
- Average latency
- Error rate
- Half-open recovery after 30 seconds

## Configuration

All config via environment variables (`.env` file):

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | required | PostgreSQL connection string |
| `REDIS_URL` | required | Redis connection URL |
| `JWT_SECRET` | required | Must match your backend |
| `NEXTJS_API_URL` | required | Backend URL for fallback AI |
| `NEXTJS_API_KEY` | required | Service-to-service auth key |
| `WEBHOOK_SECRET` | required | HMAC-SHA256 secret for webhooks |
| `PORT` | `3001` | Server port |
| `RUST_LOG` | `info` | Log level |
| `MEMORY_CACHE_SIZE` | `1000` | LRU cache capacity |
| `DB_POOL_MAX_CONNECTIONS` | `50` | Max DB connections |
| `DB_POOL_MIN_CONNECTIONS` | `10` | Min DB connections |
| `DB_ACQUIRE_TIMEOUT_SECS` | `5` | Connection acquire timeout |
| `OPENAI_API_KEY` | optional | Auto-register OpenAI on startup |
| `GROQ_API_KEY` | optional | Auto-register Groq on startup |
| `DEEPSEEK_API_KEY` | optional | Auto-register DeepSeek on startup |

## Performance

| Operation | Latency |
|-----------|---------|
| Memory cache hit | ~0.01ms |
| Redis cache hit | ~1ms |
| Database query | ~5-10ms |
| Pathfinding (100 blocks) | ~5ms |
| AI chat (template) | ~1ms |
| AI chat (LLM) | ~200-2000ms |

## Development

```bash
# Run all tests
cargo test --workspace --lib

# Build everything
cargo build --workspace --release

# Build only the API server
cargo build --release -p npc-api

# Build the CLI
cargo build --release -p blaniel-cli

# Build FFI library
cargo build --release -p blaniel-ffi

# Format
cargo fmt

# Lint
cargo clippy --workspace -- -D warnings
```

## Deployment

### Docker

```bash
docker build -t blaniel-npc-api .
docker run -d \
  -p 3001:3001 \
  -e DATABASE_URL=postgres://... \
  -e REDIS_URL=redis://... \
  -e JWT_SECRET=your-secret \
  blaniel-npc-api
```

### Environment Variables for LLM Auto-Registration

Set any of these to automatically register providers at startup:

```bash
OPENAI_API_KEY=sk-...
GROQ_API_KEY=gsk-...
DEEPSEEK_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
GEMINI_API_KEY=AIza...
```

## Documentation

For in-depth information, see the [docs/](docs/) directory:

- **[docs/OPTIMIZATIONS.md](docs/OPTIMIZATIONS.md)** - Real-time performance optimizations (multi-key load balancing, predictive caching, hybrid responses, priority queues)
- **[docs/PERFORMANCE.md](docs/PERFORMANCE.md)** - Performance benchmarks, tuning, and troubleshooting
- **[docs/ACTION-SYSTEM.md](docs/ACTION-SYSTEM.md)** - Complete AI action system guide with analytics, priorities, cooldowns, templates, and composite actions

## Author

**Lucas Dono** — Computer Science student from Argentina. Solo developer of the entire Blaniel ecosystem.

- Email: [lucasdono332@gmail.com](mailto:lucasdono332@gmail.com)
- LinkedIn: [linkedin.com/in/lucas-dono](https://www.linkedin.com/in/lucas-dono)
- GitHub: [@Lucas-Dono](https://github.com/Lucas-Dono)
- Support: [tecito.app/blaniel](https://tecito.app/blaniel)

## License

Apache 2.0 — see [LICENSE](LICENSE).

Copyright (c) 2024-2026 Lucas Dono
