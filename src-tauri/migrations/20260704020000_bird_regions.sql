-- Many-to-many bird ↔ eBird region code. A species occurs in many regions;
-- importing a country fans out to its provinces and tags each bird with every
-- region it was seen in. Region hierarchy is resolved by code prefix
-- (child code starts with "<parent>-"), so no explicit tree is needed.
CREATE TABLE IF NOT EXISTS bird_regions (
    bird_id     INTEGER NOT NULL REFERENCES birds(ID) ON DELETE CASCADE,
    region_code TEXT NOT NULL,
    PRIMARY KEY (bird_id, region_code)
);
CREATE INDEX IF NOT EXISTS idx_bird_regions_code ON bird_regions(region_code);

-- Backfill from the old single-region column.
INSERT OR IGNORE INTO bird_regions (bird_id, region_code)
  SELECT id, region FROM birds WHERE region IS NOT NULL AND region != '';
