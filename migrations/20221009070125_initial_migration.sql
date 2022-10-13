create table input_flakes (
        input_flake_id bigserial primary key,
        flake_url text not null unique
);
