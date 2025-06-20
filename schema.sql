-- Adminer 5.3.0 PostgreSQL 17.5 dump

\connect "kennels";

CREATE TABLE IF NOT EXISTS "public"."kennelings" (
    "guild_id" character varying(128) NOT NULL,
    "victim" character varying(128) NOT NULL,
    "kenneler" character varying(128) NOT NULL,
    "kennel_length" interval NOT NULL,
    "kenneled_at" timestamp NOT NULL
)
WITH (oids = false);

CREATE TABLE IF NOT EXISTS "public"."servers" (
    "guild_id" character varying(128) NOT NULL,
    "command_name" text,
    "command_verb" text,
    "release_message" text,
    "role_id" character varying(128),
    CONSTRAINT "kennels_pkey" PRIMARY KEY ("guild_id")
)
WITH (oids = false);


-- 2025-06-19 19:35:15 UTC