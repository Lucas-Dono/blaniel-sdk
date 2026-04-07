# Blaniel NPC SDK - Python Bindings

## Installation

```bash
pip install blaniel-npc
```

## Quick Start

```python
from blaniel import Blaniel

# Connect to your Blaniel NPC API
client = Blaniel("https://api.blaniel.com", "your-jwt-token")

# Chat with an NPC
response = client.chat("npc-123", "Hello, how are you?")
print(response["response"])     # The NPC's reply
print(response["emotion"])      # Current emotion
print(response["animation"])    # Suggested animation

# Navigate an NPC
client.navigate_to("npc-123", "tavern")

# Configure navigation
client.set_navigation_config("npc-123", """
{
    "mode": "hybrid",
    "scene_id": "village",
    "default_speed": 4.0
}
""")

# Set up action system for an RPG
client.set_action_config("npc-123", """
{
    "game_genre": "rpg",
    "enabled_actions": ["combat", "magic", "trading", "social"],
    "introduction": "You are a merchant in the village square.",
    "action_explain": "You can trade goods and chat with travelers.",
    "movement_explain": "You walk around the market area."
}
""")

# Add an LLM provider
client.register_llm_provider("groq", "gsk_your_key_here")

# Pygame example
import pygame

client = Blaniel("http://localhost:3001", "dev-token")

# In your game loop:
for event in pygame.event.get():
    if event.type == pygame.KEYDOWN:
        if event.key == pygame.K_e:  # Talk to nearby NPC
            nearby = client.get_nearby_npcs(player_x, player_y, player_z, 5.0, "overworld")
            for npc in nearby["npcs"]:
                response = client.chat(npc["id"], player_message)
                show_dialogue(response["response"])
```

## API Reference

### `Blaniel(api_url, api_key, timeout_ms=30000)`

Create a new client connection.

### Methods

| Method | Description |
|--------|-------------|
| `health()` | Check API health |
| `get_npc(id)` | Get NPC state |
| `chat(npc_id, message)` | Chat with NPC |
| `move_npc(npc_id, x, y, z, world)` | Move NPC |
| `navigate_to(npc_id, name)` | Navigate to named location |
| `navigate_to_coords(npc_id, x, y, z, world)` | Navigate to coordinates |
| `pathfind(npc_id, fx, fy, fz, tx, ty, tz, world)` | Calculate path |
| `get_nearby_npcs(x, y, z, radius, world)` | Find nearby NPCs |
| `get_navigation_config(npc_id)` | Get nav config |
| `set_navigation_config(npc_id, config_json)` | Set nav config |
| `get_action_config(npc_id)` | Get action config |
| `set_action_config(npc_id, config_json)` | Set action config |
| `list_llm_providers()` | List LLM providers |
| `register_llm_provider(type, key, model?, base_url?)` | Add LLM provider |
| `ambient_dialogue(ids, context)` | Generate ambient dialogue |
| `register_scene(scene_json)` | Register scene positions |
