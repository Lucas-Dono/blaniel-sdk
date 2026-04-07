using System;
using System.Collections.Generic;

namespace Blaniel.NPC
{
    public class NpcClientCache
    {
        private readonly int _maxSize;
        private readonly Dictionary<string, CacheEntry> _entries;
        private readonly LinkedList<string> _lruList;

        public NpcClientCache(int maxSize)
        {
            _maxSize = maxSize;
            _entries = new Dictionary<string, CacheEntry>(maxSize);
            _lruList = new LinkedList<string>();
        }

        public int Count => _entries.Count;

        public bool TryGet<T>(string key, out T value)
        {
            if (_entries.TryGetValue(key, out var entry))
            {
                if (!entry.IsExpired)
                {
                    _lruList.Remove(key);
                    _lruList.AddLast(key);
                    value = (T)entry.Value;
                    return true;
                }

                Remove(key);
            }

            value = default;
            return false;
        }

        public void Set<T>(string key, T value, int ttlSeconds)
        {
            if (_entries.ContainsKey(key))
            {
                Remove(key);
            }

            while (_entries.Count >= _maxSize)
            {
                EvictOldest();
            }

            var entry = new CacheEntry(value, DateTime.UtcNow.AddSeconds(ttlSeconds));
            _entries[key] = entry;
            _lruList.AddLast(key);
        }

        public void Remove(string key)
        {
            if (_entries.Remove(key))
            {
                _lruList.Remove(key);
            }
        }

        public void Clear()
        {
            _entries.Clear();
            _lruList.Clear();
        }

        private void EvictOldest()
        {
            if (_lruList.First != null)
            {
                var oldest = _lruList.First.Value;
                Remove(oldest);
            }
        }

        private class CacheEntry
        {
            public object Value { get; }
            public DateTime ExpiresAt { get; }

            public CacheEntry(object value, DateTime expiresAt)
            {
                Value = value;
                ExpiresAt = expiresAt;
            }

            public bool IsExpired => DateTime.UtcNow > ExpiresAt;
        }
    }
}
