-- Add up migration script here
ALTER TABLE agents 
        ADD COLUMN nixos_configuration_id bigint REFERENCES nixos_configurations;
