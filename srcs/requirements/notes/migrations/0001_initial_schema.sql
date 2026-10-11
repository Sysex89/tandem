-- Baseline: the schema previously created by postgres/init/02-init.sql on a fresh volume.
-- Written to be a no-op on databases that already have it, so existing volumes adopt
-- migrations without changes. Never edit an applied migration; add a new one.

-- the container - managed by notes.
CREATE TABLE IF NOT EXISTS notes (
	id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
	title VARCHAR(255) NOT NULL DEFAULT 'Untitled',
	owner_id UUID, -- auth users.id (other database, so no foreign key)
	owner_url VARCHAR(255), -- slug, unique per owner
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- enum type for share roles
DO $$ BEGIN
	CREATE TYPE access_type AS ENUM ('View', 'Edit', 'Owner');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- shares with a user, or a public link when guest_id is NULL
CREATE TABLE IF NOT EXISTS share (
	id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
	note_id UUID NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
	guest_id UUID,
	access_role access_type NOT NULL DEFAULT 'View',
	created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	CONSTRAINT unique_share UNIQUE NULLS NOT DISTINCT (note_id, guest_id)
);

-- the content - managed by editor (Yjs state).
CREATE TABLE IF NOT EXISTS note_states (
	note_id UUID PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE,
	state_vector BYTEA NOT NULL,
	last_saved_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
