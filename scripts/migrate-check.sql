-- Verify Prisma schema compatibility
-- This script checks that the expected tables and columns exist

-- Check Agent table
SELECT column_name, data_type
FROM information_schema.columns
WHERE table_name = 'Agent'
ORDER BY ordinal_position;

-- Check InternalState table
SELECT column_name, data_type
FROM information_schema.columns
WHERE table_name = 'InternalState'
ORDER BY ordinal_position;

-- Check Message table
SELECT column_name, data_type
FROM information_schema.columns
WHERE table_name = 'Message'
ORDER BY ordinal_position;

-- Verify Agent metadata contains position fields
SELECT id, name,
       metadata->>'position_x' as pos_x,
       metadata->>'position_y' as pos_y,
       metadata->>'position_z' as pos_z,
       metadata->>'world' as world
FROM "Agent"
WHERE metadata->>'position_x' IS NOT NULL
LIMIT 5;
