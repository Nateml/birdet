-- Cache of scientific name -> common name for species that aren't in the eBird
-- taxonomy cache under that binomial. Xeno-Canto's `also` (background) field uses
-- IOC taxonomy, which diverges from eBird/Clements after genus renames and
-- lumps/splits (e.g. IOC "Mirafra africana" is eBird "Corypha africana"). We
-- resolve those names from Xeno-Canto's own English name and cache them here so
-- background species can be shown by, and excluded from options by, common name.
CREATE TABLE IF NOT EXISTS species_names (
    scientific_name TEXT PRIMARY KEY,
    common_name     TEXT NOT NULL
);

-- Seed the South African pack's two divergent background species.
INSERT OR IGNORE INTO species_names (scientific_name, common_name) VALUES
    ('Mirafra africana', 'Rufous-naped Lark'),
    ('Acrocephalus baeticatus', 'African Reed Warbler');
