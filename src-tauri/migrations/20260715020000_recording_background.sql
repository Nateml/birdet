-- Xeno-Canto's `also` field: other species audible in the background of a
-- recording, as a JSON array of scientific names. Used to (a) never offer a
-- background species as a wrong multiple-choice option, and (b) optionally
-- show the user what else is in the clip.
ALTER TABLE recordings ADD COLUMN background_json TEXT;

-- Backfill the seeded South African pack recordings (fetched from XC).
UPDATE recordings SET background_json = '["Acrocephalus baeticatus"]' WHERE xc_id = '507084';
UPDATE recordings SET background_json = '["Mirafra africana","Pogoniulus pusillus","Crithagra scotops"]' WHERE xc_id = '370472';
UPDATE recordings SET background_json = '["Zosterops virens"]' WHERE xc_id = '1155170';
