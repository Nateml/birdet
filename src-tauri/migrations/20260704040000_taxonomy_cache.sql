-- Local cache of the full eBird species taxonomy (~11k rows), so birds can be
-- searched and added by name without a per-query eBird endpoint (there isn't
-- one). Populated on first search from /ref/taxonomy/ebird. taxon_order holds
-- the eBird order name (e.g. "Passeriformes") to mirror birds.taxon_order.
CREATE TABLE IF NOT EXISTS taxonomy (
    ebird_code      TEXT PRIMARY KEY,
    common_name     TEXT NOT NULL,
    scientific_name TEXT NOT NULL,
    family          TEXT,
    family_sci      TEXT,
    taxon_order     TEXT
);
CREATE INDEX IF NOT EXISTS idx_taxonomy_common ON taxonomy(common_name);
CREATE INDEX IF NOT EXISTS idx_taxonomy_sci ON taxonomy(scientific_name);
