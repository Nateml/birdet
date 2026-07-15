-- South African starter pack: 10 common, vocal SA birds with Xeno-Canto
-- recordings (all q:A, recorded in South Africa) and full CC attribution.
-- Replaces the old placeholder "Development Pack (Test Data)" seed.
--
-- Birds are stored under their eBird taxonomy (common_name, scientific_name,
-- ebird_code) so a later region/name import of the same species dedups against
-- the seed (import keys on ebird_code, then scientific_name) instead of creating
-- a duplicate. IDs are never hardcoded: birds INSERT OR IGNORE, recordings link
-- by scientific_name, the pack links by xc_id — collision-safe on a populated DB.

-- 1. Remove the old placeholder seed (birds 1-5, recordings 1-6, dev pack).
DELETE FROM pack_recordings WHERE pack_ID = 'dev_pack_001';
DELETE FROM history   WHERE recording_ID IN (1,2,3,4,5,6);
DELETE FROM mastery   WHERE bird_ID   IN (1,2,3,4,5);
DELETE FROM recordings WHERE ID IN (1,2,3,4,5,6);
DELETE FROM birds     WHERE ID IN (1,2,3,4,5);
DELETE FROM packs     WHERE ID = 'dev_pack_001';

-- 2. The pack.
INSERT OR IGNORE INTO packs (ID, name, description) VALUES
  ('starter_sa_10', 'South African Starter', 'Ten common, vocal birds of South African gardens, bushveld and wetlands — a starter set to learn by ear. Recordings from Xeno-Canto.');

-- 3. The birds, under eBird taxonomy (skipped if the species already exists).
INSERT OR IGNORE INTO birds (common_name, scientific_name, description, ebird_code, family, family_sci, taxon_order, region) VALUES
  ('Hadada Ibis', 'Bostrychia hagedash', 'A large, common ibis of sub-Saharan Africa, famous across South African suburbs for its loud, raucous ''haa-haa-haa-de-dah'' call given in flight at dawn and dusk.', 'hadibi1', 'Ibises and Spoonbills', 'Threskiornithidae', 'Pelecaniformes', 'ZA'),
  ('African Fish-Eagle', 'Icthyophaga vocifer', 'An iconic African raptor of rivers, lakes and estuaries. Its ringing, gull-like ''weee-ah, hyo-hyo-hyo'' call, often given with the head thrown back, is emblematic of the African wild.', 'affeag1', 'Hawks, Eagles, and Kites', 'Accipitridae', 'Accipitriformes', 'ZA'),
  ('Cape Robin-Chat', 'Dessonornis caffer', 'A familiar garden bird of southern Africa with an orange throat and breast. Its cheerful, warbling dawn song and a distinctive ''cherry-berry'' alarm note are widely heard.', 'carcha1', 'Old World Flycatchers', 'Muscicapidae', 'Passeriformes', 'ZA'),
  ('Bokmakierie', 'Telophorus zeylonus', 'A colourful bush-shrike of scrub and gardens, olive-green above with a bright yellow underside and black collar. Named for its loud, ringing, antiphonal duet ''bok-bok-kik''.', 'bokmak1', 'Bushshrikes and Allies', 'Malaconotidae', 'Passeriformes', 'ZA'),
  ('Blacksmith Lapwing', 'Vanellus armatus', 'A boldly pied black, white and grey wader of wetlands and grassland. Named for its sharp, metallic ''tink, tink, tink'' alarm call, like a blacksmith striking an anvil.', 'blaplo1', 'Plovers and Lapwings', 'Charadriidae', 'Charadriiformes', 'ZA'),
  ('Ring-necked Dove', 'Streptopelia capicola', 'An abundant dove of woodland, savanna and gardens across southern Africa. Its persistent three-note ''kuk-COORRR-uk'' (often rendered ''work HARD-er'') is one of the region''s most familiar sounds.', 'rindov', 'Pigeons and Doves', 'Columbidae', 'Columbiformes', 'ZA'),
  ('Fork-tailed Drongo', 'Dicrurus adsimilis', 'A glossy black, fork-tailed bird known for its aggressive mobbing and remarkable vocal mimicry, including imitating alarm calls to steal food from other animals.', 'fotdro5', 'Drongos', 'Dicruridae', 'Passeriformes', 'ZA'),
  ('Southern Masked-Weaver', 'Ploceus velatus', 'A common weaver of gardens and wetlands; breeding males are bright yellow with a black face mask and red eye. Colonies are noisy with swizzling, chattering song.', 'afmwea', 'Weavers and Allies', 'Ploceidae', 'Passeriformes', 'ZA'),
  ('Cape White-eye', 'Zosterops virens', 'A small, active greenish bird with a conspicuous white eye-ring, moving through gardens and woodland in busy flocks with soft, warbling, tinkling calls.', 'capwhe2', 'White-eyes, Yuhinas, and Allies', 'Zosteropidae', 'Passeriformes', 'ZA'),
  ('Olive Thrush', 'Turdus olivaceus', 'One of southern Africa''s most common garden thrushes, olive-brown with an orange bill and belly. It sings a rich, fluty dawn song and forages for worms on lawns.', 'olithr2', 'Thrushes and Allies', 'Turdidae', 'Passeriformes', 'ZA');

-- 4. One recording per bird, linked by eBird scientific_name.
INSERT INTO recordings (bird_ID, filename, date_recorded, location, source, xc_id, recordist, license_url, quality, rec_type)
  SELECT b.ID, '01_hadada_ibis.wav', '2026-05-11', 'Kruger Park (near  Skukuza), Ehlanzeni District Municipality, Mpumalanga', 'xeno-canto', '1146089', 'African Bioacoustics Community', 'https://creativecommons.org/licenses/by-nc-sa/4.0/', 'A', 'flight call' FROM birds b WHERE b.scientific_name = 'Bostrychia hagedash'
  UNION ALL
  SELECT b.ID, '02_african_fish_eagle.mp3', '2009-12-23', 'Pafuri Picnic Area-Kruger, Limpopo', 'xeno-canto', '61995', 'Charles Hesse', 'https://creativecommons.org/licenses/by-nc-nd/2.5/', 'A', 'call' FROM birds b WHERE b.scientific_name = 'Icthyophaga vocifer'
  UNION ALL
  SELECT b.ID, '03_cape_robin_chat.wav', '2026-05-31', 'Breede River DC (near  Montagu), Cape Winelands District Municipality, Western Cape', 'xeno-canto', '1149907', 'African Bioacoustics Community', 'https://creativecommons.org/licenses/by-nc-sa/4.0/', 'A', 'call' FROM birds b WHERE b.scientific_name = 'Dessonornis caffer'
  UNION ALL
  SELECT b.ID, '04_bokmakierie.mp3', '2010-07-23', 'De Hoop Nature Reserve', 'xeno-canto', '62561', 'Charles Hesse', 'https://creativecommons.org/licenses/by-nc-nd/2.5/', 'A', 'song' FROM birds b WHERE b.scientific_name = 'Telophorus zeylonus'
  UNION ALL
  SELECT b.ID, '05_blacksmith_lapwing.wav', '2025-09-20', 'City of Cape Town (near  Cape Town), City of Cape Town Metropolitan Municipality, Western Cape', 'xeno-canto', '1041281', 'African Bioacoustics Community', 'https://creativecommons.org/licenses/by-nc-sa/4.0/', 'A', 'call' FROM birds b WHERE b.scientific_name = 'Vanellus armatus'
  UNION ALL
  SELECT b.ID, '06_ring_necked_dove.wav', '2026-05-17', 'Kruger Park (near  Skukuza), Ehlanzeni District Municipality, Mpumalanga', 'xeno-canto', '1149064', 'African Bioacoustics Community', 'https://creativecommons.org/licenses/by-nc-sa/4.0/', 'A', 'call' FROM birds b WHERE b.scientific_name = 'Streptopelia capicola'
  UNION ALL
  SELECT b.ID, '07_fork_tailed_drongo.mp3', '2025-06-29', 'Pretoriuskop Rest Camp, Kruger National Park, Mpumalanga', 'xeno-canto', '1038565', 'Bobby Wilcox', 'https://creativecommons.org/licenses/by-nc-sa/4.0/', 'A', 'call' FROM birds b WHERE b.scientific_name = 'Dicrurus adsimilis'
  UNION ALL
  SELECT b.ID, '08_southern_masked_weaver.mp3', '2019-11-16', 'City of Matlosana (near  Klerksdorp), Southern DC, North West', 'xeno-canto', '507084', 'Tony Archer', 'https://creativecommons.org/licenses/by-nc-sa/4.0/', 'A', 'call' FROM birds b WHERE b.scientific_name = 'Ploceus velatus'
  UNION ALL
  SELECT b.ID, '09_cape_white_eye.mp3', '2017-05-17', 'Port Alfred, Western District, Eastern Cape', 'xeno-canto', '370472', 'Tim Cockcroft', 'https://creativecommons.org/licenses/by-nc-sa/4.0/', 'A', 'call' FROM birds b WHERE b.scientific_name = 'Zosterops virens'
  UNION ALL
  SELECT b.ID, '10_olive_thrush.wav', '2026-03-20', 'Kannaland Local Municipality (near  Vyversrus), Garden Route District Municipality, Western Cape', 'xeno-canto', '1155170', 'Walter Wallner', 'https://creativecommons.org/licenses/by-nc-sa/4.0/', 'A', 'dawn song' FROM birds b WHERE b.scientific_name = 'Turdus olivaceus';

-- 5. Link every seeded recording into the pack, by xc_id.
INSERT OR IGNORE INTO pack_recordings (pack_ID, recording_ID)
  SELECT 'starter_sa_10', r.ID FROM recordings r
  WHERE r.source = 'xeno-canto' AND r.xc_id IN ('1146089', '61995', '1149907', '62561', '1041281', '1149064', '1038565', '507084', '370472', '1155170');
