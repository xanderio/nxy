create table input_flakes (
        input_flake_id bigserial primary key,
        flake_url text not null unique,
        description text,
        path text not null,
        revision text not null,
        last_modified timestamp with time zone not null,
        url text not null,
        locks jsonb not null
);
