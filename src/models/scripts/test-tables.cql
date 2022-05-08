CREATE TABLE IF NOT EXISTS bots (
    id bigint,
    username text,
    avatar text,
    discriminator int,
    prefix text,
    is_packable boolean,
    slug text,
    flags bigint,
    features bigint,
    tags set<text>,
    invite_url text,
    created_on timestamp,
    is_hidden boolean,
    is_forced_into_hiding boolean,
    owner_id bigint,
    co_owner_ids set<bigint>,
    guild_count int,
    brief_description text,
    PRIMARY KEY ( id )
);
CREATE TABLE IF NOT EXISTS bot_votes (
    id bigint,
    votes counter,
    all_time_votes counter,
    PRIMARY KEY ( id )
);
CREATE TABLE IF NOT EXISTS packs (
    id bigint,
    name text,
    description text,
    created_on timestamp,
    tag text,
    bots set<bigint>,
    is_hidden boolean,
    is_forced_into_hiding boolean,
    owner_id bigint,
    co_owner_ids set<bigint>,
    PRIMARY KEY ( id )
);
CREATE TABLE IF NOT EXISTS pack_likes (
    id bigint,
    likes counter,
    PRIMARY KEY ( id )
);
