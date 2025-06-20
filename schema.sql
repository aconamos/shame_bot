-- Adminer 5.3.0 PostgreSQL 17.5 dump

\connect "kennels";

CREATE TABLE "public"."kennelings" (
    "guild_id" character varying(128) NOT NULL,
    "victim" character varying(128) NOT NULL,
    "kennel_length" interval NOT NULL,
    "kenneled_at" timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "kenneler" character varying(128) NOT NULL,
    "released_at" timestamp GENERATED ALWAYS AS ((kenneled_at + kennel_length)) STORED NOT NULL
)
WITH (oids = false);


CREATE TABLE "public"."servers" (
    "guild_id" character varying(128) NOT NULL,
    "command_name" text DEFAULT 'kennel' NOT NULL,
    "command_verb" text DEFAULT 'They will be released $return.''' NOT NULL,
    "release_message" text DEFAULT '$victim has been released from the kennel.' NOT NULL,
    "role_id" character varying(128) NOT NULL,
    CONSTRAINT "kennels_pkey" PRIMARY KEY ("guild_id")
)
WITH (oids = false);

CREATE INDEX servers_guild_id_command_name ON public.servers USING btree (guild_id, command_name);


-- 2025-06-20 05:53:19 UTC