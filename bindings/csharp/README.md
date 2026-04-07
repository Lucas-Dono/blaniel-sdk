# Blaniel NPC SDK - Unity/C# Bindings

## Installation

1. Copy `BlanielClient.cs` into your Unity project's `Assets/Scripts/` folder
2. No native dependencies required - pure HTTP client

## Quick Start

```csharp
using Blaniel.NPC;

public class NPCManager : MonoBehaviour
{
    private BlanielClient _client;

    void Start()
    {
        _client = new BlanielClient("https://api.blaniel.com", "your-jwt-token");
    }

    void OnDestroy()
    {
        _client?.Dispose();
    }

    public async void TalkToNPC(string npcId, string message)
    {
        var response = await _client.Chat(npcId, message);
        Debug.Log($"NPC says: {response.Response} (emotion: {response.Emotion})");
    }

    public async void MoveToLocation(string npcId, string locationName)
    {
        var result = await _client.NavigateTo(npcId, locationName);
        if (result.Success)
        {
            Debug.Log($"NPC moving to {result.TargetName}, distance: {result.Distance}");
        }
    }

    public async void SetupScene()
    {
        // Register scene positions
        await _client.RegisterScene("village", "overworld", new[]
        {
            new ScenePosition
            {
                Name = "tavern",
                Position = new Position(100, 64, 200),
                Aliases = new List<string> { "bar", "inn" },
                Tags = new List<string> { "building", "social" }
            },
            new ScenePosition
            {
                Name = "market",
                Position = new Position(150, 64, 180),
                Aliases = new List<string> { "shop", "store" }
            }
        });

        // Configure NPC for RPG
        await _client.SetNavigationConfig("npc-123", new NavigationConfig
        {
            Mode = "hybrid",
            SceneId = "village"
        });
    }
}
```

## Coroutine Example (Unity-friendly)

```csharp
using UnityEngine;
using System.Collections;

public class NPCChat : MonoBehaviour
{
    [SerializeField] private string apiUrl = "http://localhost:3001";
    [SerializeField] private string apiKey = "dev-token";
    [SerializeField] private string npcId = "npc-123";

    private BlanielClient _client;

    void Start()
    {
        _client = new BlanielClient(apiUrl, apiKey);
        StartCoroutine(ChatLoop());
    }

    IEnumerator ChatLoop()
    {
        var task = _client.Chat(npcId, "What's happening today?");
        yield return new WaitUntil(() => task.IsCompleted);

        if (task.IsCompletedSuccessfully)
        {
            Debug.Log(task.Result.Response);
        }
    }
}
```
