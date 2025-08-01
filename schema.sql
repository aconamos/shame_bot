-- Adminer 5.3.0 PostgreSQL 17.5 dump

DROP TABLE IF EXISTS "kennelings";
DROP SEQUENCE IF EXISTS kennelings_id_seq;
CREATE SEQUENCE kennelings_id_seq INCREMENT 1 MINVALUE 1 MAXVALUE 2147483647 CACHE 1;

CREATE TABLE "public"."kennelings" (
    "guild_id" character varying(128) NOT NULL,
    "victim_id" character varying(128) NOT NULL,
    "kennel_length" interval NOT NULL,
    "kenneled_at" timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "author_id" character varying(128) NOT NULL,
    "released_at" timestamp GENERATED ALWAYS AS ((kenneled_at + kennel_length)) STORED NOT NULL,
    "id" integer DEFAULT nextval('kennelings_id_seq') NOT NULL,
    CONSTRAINT "kennelings_pkey" PRIMARY KEY ("id")
)
WITH (oids = false);

CREATE INDEX kennelings_released_at ON public.kennelings USING btree (released_at);


DROP TABLE IF EXISTS "servers";
CREATE TABLE "public"."servers" (
    "guild_id" character varying(128) NOT NULL,
    "command_name" text DEFAULT 'kennel' NOT NULL,
    "announcement_message" text DEFAULT '$victim has been locked away in the kennel.' NOT NULL,
    "release_message" text DEFAULT '$victim has been released from the kennel.' NOT NULL,
    "role_id" character varying(128) NOT NULL,
    "kennel_channel" character varying(128),
    "kennel_message" text DEFAULT 'You will return $return.''' NOT NULL,
    CONSTRAINT "kennels_pkey" PRIMARY KEY ("guild_id")
)
WITH (oids = false);

CREATE INDEX servers_guild_id_command_name ON public.servers USING btree (guild_id, command_name);


-- 2025-07-31 00:38:25 UTC