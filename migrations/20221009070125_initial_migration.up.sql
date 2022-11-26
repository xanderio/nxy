create table flakes(
        flake_id bigint generated always as identity primary key,
        flake_url text not null unique
);

create table flake_revisions (
	flake_revision_id bigint generated always as identity primary key,
	flake_id bigint references flakes,
        revision text not null, 
        last_modified timestamp with time zone not null,
        url text not null,
        metadata jsonb not null
);

create table nixos_configurations (
	nixos_configuration_id bigint generated always as identity primary key,
	flake_revision_id bigint references flake_revisions,
	name text not null,
	path text not null
);

create table agents (
	agent_id uuid primary key,
        current_system text
);
