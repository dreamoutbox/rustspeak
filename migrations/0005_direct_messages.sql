CREATE TABLE dm_conversations (
    id UUID PRIMARY KEY,
    user_a UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_b UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT dm_conversations_ordered_pair CHECK (user_a < user_b),
    CONSTRAINT dm_conversations_unique_pair UNIQUE (user_a, user_b)
);

CREATE INDEX dm_conversations_user_a_idx ON dm_conversations (user_a);
CREATE INDEX dm_conversations_user_b_idx ON dm_conversations (user_b);

CREATE TABLE dm_messages (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES dm_conversations(id) ON DELETE CASCADE,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX dm_messages_conv_id_idx ON dm_messages (conversation_id, id DESC);

