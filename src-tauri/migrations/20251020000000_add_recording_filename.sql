-- Add a filename column so recording_id maps deterministically to a bundled audio file
-- (files live in src-tauri/resources/recordings/, bundled via tauri.conf.json).

ALTER TABLE recordings ADD COLUMN filename VARCHAR(255);

UPDATE recordings SET filename = '1_peregrine_falcon.wav' WHERE ID = 1;
UPDATE recordings SET filename = '2_peregrine_falcon.wav' WHERE ID = 2;
UPDATE recordings SET filename = '3_bald_eagle.mp3'       WHERE ID = 3;
UPDATE recordings SET filename = '4_european_robin.mp3'   WHERE ID = 4;
UPDATE recordings SET filename = '5_great_horned_owl.wav' WHERE ID = 5;
UPDATE recordings SET filename = '6_scarlet_macaw.mp3'    WHERE ID = 6;
