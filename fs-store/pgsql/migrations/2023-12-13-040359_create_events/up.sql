-- Your SQL goes here
create table events (
    cid varchar(70) not null
        constraint events_pk
            primary key,
    prev varchar(70),
    genesis varchar(70) not null,
    blocks bytea[] not null check (blocks <> '{}' and array_position(blocks, null) is null)
);

create table streams (
    stream_id varchar(70) not null
        constraint streams_pk
            primary key,
    dapp_id uuid not null,
    tip varchar(70) not null,
    account varchar(100),
    model_id varchar(70),
    content jsonb not null
);