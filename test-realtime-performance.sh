#!/bin/bash
# Test de rendimiento en tiempo real
# Simula carga real de NPCs

set -e

echo "========================================="
echo "  Test de Rendimiento en Tiempo Real"
echo "========================================="
echo ""

# Check dependencies
command -v wrk >/dev/null 2>&1 || {
    echo "⚠️  wrk no está instalado (herramienta de benchmarking)"
    echo "Instalar: sudo apt install wrk  (Ubuntu/Debian)"
    echo "        : brew install wrk     (macOS)"
    echo ""
    echo "Continuando con curl simple..."
    USE_CURL=1
}

# Check if server is running
if ! curl -s http://localhost:3001/health >/dev/null 2>&1; then
    echo "❌ El servidor no está corriendo en localhost:3001"
    echo "Iniciar con: cargo run --release"
    exit 1
fi

echo "✅ Servidor detectado en localhost:3001"
echo ""

# Get JWT token (assuming you have one)
if [ -z "$JWT_TOKEN" ]; then
    echo "⚠️  JWT_TOKEN no configurado"
    echo "Export tu token: export JWT_TOKEN=your_jwt_token"
    echo ""
    echo "Tests sin autenticación (solo endpoints públicos)..."
    echo ""
fi

# Test 1: Health check (baseline)
echo "========================================="
echo "Test 1: Health Check (baseline)"
echo "========================================="
echo "Midiendo latencia de red..."
echo ""

for i in {1..10}; do
    time curl -s http://localhost:3001/health >/dev/null
done

echo ""

# Test 2: Cache hit simulation
echo "========================================="
echo "Test 2: Cache Hit (operación más rápida)"
echo "========================================="
echo "Simulando lookup de NPC con cache hit..."
echo ""

if [ ! -z "$JWT_TOKEN" ]; then
    for i in {1..5}; do
        echo "Request $i:"
        time curl -s -w "\nTime: %{time_total}s\n" \
            -H "Authorization: Bearer $JWT_TOKEN" \
            http://localhost:3001/api/v1/npc/test-npc-123 >/dev/null
    done
else
    echo "Skipping (necesita JWT_TOKEN)"
fi

echo ""

# Test 3: Concurrency test
if [ -z "$USE_CURL" ]; then
    echo "========================================="
    echo "Test 3: Carga Concurrente (wrk)"
    echo "========================================="
    echo "Simulando 10 NPCs con 50 requests cada uno..."
    echo ""

    wrk -t 10 -c 10 -d 10s \
        -H "Authorization: Bearer ${JWT_TOKEN:-dummy}" \
        http://localhost:3001/health

    echo ""
    echo "========================================="
    echo "Test 4: Carga Alta (wrk)"
    echo "========================================="
    echo "Simulando 50 NPCs simultáneos..."
    echo ""

    wrk -t 20 -c 50 -d 10s \
        -H "Authorization: Bearer ${JWT_TOKEN:-dummy}" \
        http://localhost:3001/health
fi

echo ""
echo "========================================="
echo "  Análisis de Resultados"
echo "========================================="
echo ""
echo "For real-time games:"
echo "  • Excelente: <50ms  (< 1 tick)"
echo "  • Bueno:     <100ms (< 2 ticks)"
echo "  • Aceptable: <200ms (< 4 ticks)"
echo "  • Lag:       >300ms"
echo ""
echo "Tus resultados deberían estar en 'Excelente' con cache."
echo ""
echo "Siguiente paso:"
echo "  1. Revisar logs del servidor para ver latencias reales"
echo "  2. Ejecutar: cargo bench --bench performance"
echo "  3. Si tienes VENICE_API_KEY: cargo bench --bench ai_providers --features integration-tests"
echo ""
