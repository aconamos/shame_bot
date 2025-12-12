-- Adminer 5.4.0 PostgreSQL 17.5 dump

DROP TABLE IF EXISTS "kennelings";
DROP SEQUENCE IF EXISTS kennelings_id_seq;
CREATE SEQUENCE kennelings_id_seq INCREMENT 1 MINVALUE 1 MAXVALUE 2147483647 CACHE 1;

CREATE TABLE "public"."kennelings" (
    "id" integer DEFAULT nextval('kennelings_id_seq') NOT NULL,
    "kennel_id" integer NOT NULL,
    "author_id" bigint NOT NULL,
    "victim_id" bigint NOT NULL,
    "kenneled_at" date DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "kennel_length" interval NOT NULL,
    "released_at" timestamp GENERATED ALWAYS AS ((kenneled_at + kennel_length)) STORED NOT NULL,
    "msg_announce_id" bigint,
    "kennel_msg_id" bigint,
    CONSTRAINT "kennelings_pkey" PRIMARY KEY ("id")
)
WITH (oids = false);


DROP TABLE IF EXISTS "kennels";
DROP SEQUENCE IF EXISTS kennels_id_seq;
CREATE SEQUENCE kennels_id_seq INCREMENT 1 MINVALUE 1 MAXVALUE 2147483647 CACHE 1;

CREATE TABLE "public"."kennels" (
    "id" integer DEFAULT nextval('kennels_id_seq') NOT NULL,
    "command" text NOT NULL,
    "guild_id" bigint NOT NULL,
    "role_id" bigint NOT NULL,
    "msg_announce" text,
    "msg_announce_edit" text,
    "msg_release" text,
    "kennel_channel_id" bigint,
    "kennel_msg" text,
    "kennel_msg_edit" text,
    "kennel_release_msg" text,
    "opt_in_to_metrics" boolean DEFAULT false NOT NULL,
    CONSTRAINT "kennels_pkey" PRIMARY KEY ("id")
)
WITH (oids = false);

CREATE UNIQUE INDEX kennels_role_id_key ON public.kennels USING btree (role_id);

CREATE UNIQUE INDEX kennels_name_key ON public.kennels USING btree (command);


DROP TABLE IF EXISTS "sent_messages";
CREATE TABLE "public"."sent_messages" (
    "message_id" bigint NOT NULL,
    "channel_id" bigint NOT NULL,
    CONSTRAINT "sent_messages_message_id_pkey" PRIMARY KEY ("message_id")
)
WITH (oids = false);

CREATE UNIQUE INDEX sent_messages_message_id_key ON public.sent_messages USING btree (message_id);


ALTER TABLE ONLY "public"."kennelings" ADD CONSTRAINT "kennelings_kennel_id_fkey" FOREIGN KEY (kennel_id) REFERENCES kennels(id) NOT DEFERRABLE;
ALTER TABLE ONLY "public"."kennelings" ADD CONSTRAINT "kennelings_kennel_msg_id_fkey" FOREIGN KEY (kennel_msg_id) REFERENCES sent_messages(message_id) NOT DEFERRABLE;
ALTER TABLE ONLY "public"."kennelings" ADD CONSTRAINT "kennelings_msg_announce_id_fkey" FOREIGN KEY (msg_announce_id) REFERENCES sent_messages(message_id) NOT DEFERRABLE;

-- 2025-12-12 05:05:22 UTC