-- Region membership is now resolved on demand from eBird's spplist endpoint
-- (authoritative, hierarchy handled server-side), so the provenance-based
-- bird_regions table is gone. We keep a small cache of the eBird region tree
-- to back the region picker without re-hitting the API every time.
DROP INDEX IF EXISTS idx_bird_regions_code;
DROP TABLE IF EXISTS bird_regions;

CREATE TABLE IF NOT EXISTS regions (
    code   TEXT PRIMARY KEY NOT NULL,
    name   TEXT NOT NULL,
    parent TEXT,                 -- parent region code; 'world' for countries
    level  TEXT NOT NULL         -- 'country' | 'subnational1' | 'subnational2'
);
