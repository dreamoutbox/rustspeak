CREATE TYPE user_role AS ENUM ('member', 'admin', 'owner');

ALTER TABLE users
    ADD COLUMN role user_role NOT NULL DEFAULT 'member';

CREATE UNIQUE INDEX users_single_owner_idx ON users (role) WHERE role = 'owner';

