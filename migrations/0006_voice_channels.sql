CREATE TABLE voice_channels (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX voice_channels_name_lower_idx ON voice_channels (LOWER(name));

