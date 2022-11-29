CREATE TABLE flakes (
        flake_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
        flake_url TEXT NOT NULL UNIQUE
);

CREATE TABLE flake_revisions (
	flake_revision_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	flake_id BIGINT REFERENCES flakes NOT NULL,
        revision TEXT NOT NULL, 
        last_modified TIMESTAMP WITH TIME ZONE NOT NULL,
        url TEXT NOT NULL,
        metadata JSONB NOT NULL
);

CREATE TABLE nixos_configurations (
	nixos_configuration_id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	flake_id BIGINT NOT NULL REFERENCES flakes,
	name TEXT NOT NULL,

        UNIQUE (flake_id, name)
);

CREATE TABLE nixos_configuration_evaluations (
	flake_revision_id BIGINT REFERENCES flake_revisions, 
	nixos_configuration_id BIGINT REFERENCES nixos_configurations,
	store_path TEXT NOT NULL,

	PRIMARY KEY (flake_revision_id, nixos_configuration_id)
);

CREATE TABLE agents (
	agent_id UUID PRIMARY KEY,
        current_system TEXT
);
