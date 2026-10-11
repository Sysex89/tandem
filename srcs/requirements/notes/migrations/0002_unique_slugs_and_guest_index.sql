-- Slugs were unique per owner only by an application check, which concurrent creates
-- could both pass. Renumber any duplicates that already exist (oldest keeps the slug),
-- then let the database enforce it. The index also serves "notes of owner X" queries.
UPDATE notes n
SET owner_url = n.owner_url || '-' || left(n.id::text, 8)
FROM (
	SELECT id, row_number() OVER (PARTITION BY owner_id, owner_url ORDER BY created_at, id) AS rn
	FROM notes
	WHERE owner_url IS NOT NULL
) dup
WHERE n.id = dup.id AND dup.rn > 1;

CREATE UNIQUE INDEX notes_owner_slug_key ON notes (owner_id, owner_url);

-- "shared with me" and the editor's access check look shares up by guest
CREATE INDEX share_guest_id_idx ON share (guest_id);
