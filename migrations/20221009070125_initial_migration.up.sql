create table flakes (
        flake_id bigint generated always as identity primary key,
        flake_url text not null unique
);

create table flake_revisions (
	flake_revision_id bigint generated always as identity primary key,
	flake_id bigint references flakes NOT NULL,
        revision text not null, 
        last_modified timestamp with time zone not null,
        url text not null,
        metadata jsonb not null
);

create table nixos_configurations (
	nixos_configuration_id bigint generated always as identity primary key,
	flake_id bigint not null references flakes,
	name text not null,

        unique (flake_id, name)
);

create table nixos_configuration_evaluations (
	flake_revision_id bigint references flake_revisions, 
	nixos_configuration_id bigint references nixos_configurations,
	store_path text not null,

	primary key (flake_revision_id, nixos_configuration_id)
);

create table agents (
	agent_id uuid primary key,
        current_system text
);
