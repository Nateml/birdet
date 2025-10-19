-- Add migration script here

INSERT INTO birds (ID, common_name, scientific_name, description) VALUES
(1, 'Peregrine Falcon', 'Falco peregrinus', 'The peregrine falcon is known for its incredible speed, reaching over 200 mph during its hunting stoop (high-speed dive). It is a widespread bird of prey found on every continent except Antarctica.'),
(2, 'Bald Eagle', 'Haliaeetus leucocephalus', 'The bald eagle is a bird of prey found in North America. It is the national bird and symbol of the United States. Known for its white head and tail feathers, it primarily feeds on fish.'),
(3, 'European Robin', 'Erithacus rubecula', 'The European robin is a small insectivorous passerine bird that is widely distributed across Europe. It is easily recognizable by its orange-red breast and face, contrasting with its brown upperparts and grey belly.'),
(4, 'Great Horned Owl', 'Bubo virginianus', 'The great horned owl is a large owl native to the Americas. It is known for its distinctive ear tufts, yellow eyes, and deep hooting voice. It is a powerful predator, feeding on a variety of prey including mammals, birds, and reptiles.'),
(5, 'Scarlet Macaw', 'Ara macao', 'The scarlet macaw is a large, colorful parrot native to Central and South America. It is known for its bright red, yellow, and blue plumage. Scarlet macaws are social birds that often live in pairs or small flocks and are popular in aviculture due to their striking appearance and intelligence.');

INSERT INTO RECORDINGS (ID, bird_ID, date_recorded, location) VALUES
    (1, 1, "2021-02-21", "Greater London, England"),
    (2, 1, "2017-07-09", "Essex, England"),
    (3, 2, "2018-06-29", "Alaska, USA"),
    (4, 3, "2025-10-17", "Vlaamse Gewest, Belgium"),
    (5, 4, "2024-11-05", "California, USA"),
    (6, 5, "2025-02-02", "Ecuador");

INSERT INTO packs (ID, name, description) VALUES
    ('dev_pack_001', 'Development Pack (Test Data)', 'A pack created for development and testing purposes, containing a selection of bird recordings.');

INSERT INTO pack_recordings (pack_ID, recording_ID) VALUES
    ('dev_pack_001', 1),
    ('dev_pack_001', 2),
    ('dev_pack_001', 3),
    ('dev_pack_001', 4),
    ('dev_pack_001', 5),
    ('dev_pack_001', 6);

