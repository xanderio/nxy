-- Add down migration script here
ALTER TABLE agents
	DROP COLUMN nixos_configuration_id;
