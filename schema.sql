CREATE TABLE IF NOT EXISTS "system_config"
(
    key         TEXT not null
        primary key,
    value       TEXT not null,
    description TEXT,
    updated_at  DATETIME default CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS "token"
(
    action     text                               not null,
    user_id    integer                            not null,
    token      text              unique                 not null,
    created_at DATETIME default CURRENT_TIMESTAMP not null
);

CREATE TABLE IF NOT EXISTS "user_attribute"
(
    id    integer not null,
    attr  text    not null,
    value text    not null,
    constraint users_attribute_pk
        primary key (id, attr)
);
CREATE TABLE IF NOT EXISTS "user_avatar"
(
    id        integer                            not null
        constraint users_avatar_pk
            primary key,
    data      blob                               not null,
    update_at datetime default current_timestamp not null
);
CREATE TABLE IF NOT EXISTS "access_token"
(
    token       text                 not null
        constraint access_tokens_pk
            primary key,
    description text                 not null,
    created_at  datetime             not null,
    expires_at  datetime             not null,
    user_id     integer              not null,
    category    TEXT DEFAULT 'admin' NOT NULL
);
CREATE TABLE IF NOT EXISTS "user_relation"
(
    user_id    INTEGER                            NOT NULL,
    target_id  INTEGER                            NOT NULL,
    relation   TEXT                               NOT NULL CHECK (relation IN ('follow', 'block')),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (user_id, target_id),
    FOREIGN KEY (user_id) REFERENCES "user" (id),
    FOREIGN KEY (target_id) REFERENCES "user" (id)
);
CREATE TABLE IF NOT EXISTS "notification"
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id    INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    category   TEXT    NOT NULL,
    link_id    INTEGER NOT NULL,
    content    TEXT    NOT NULL,
    meta       TEXT     DEFAULT NULL,
    FOREIGN KEY (user_id) REFERENCES "user" (id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS "page"
(
    id           INTEGER
        primary key autoincrement,
    path         TEXT not null
        unique,
    title        TEXT not null,
    description  TEXT     default '',
    content_type TEXT     default 'markdown',
    content      TEXT not null,
    created_at   DATETIME default CURRENT_TIMESTAMP,
    updated_at   DATETIME default CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS "email_queue"
(
    id            INTEGER primary key autoincrement,
    created_at    DATETIME default CURRENT_TIMESTAMP not null,
    user_id       INTEGER  default 0                 not null,
    email_from    TEXT                               not null,
    email_to      TEXT                               not null,
    email_subject TEXT                               not null,
    email_body    TEXT                               not null,
    result        TEXT     default 'pending'         not null
);
CREATE TABLE IF NOT EXISTS "invitation_code"
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    code       TEXT NOT NULL UNIQUE,
    quota      INTEGER  DEFAULT 1,
    used_count INTEGER  DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    expired_at DATETIME
);
CREATE TABLE invitation_usage
(
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    invitation_id INTEGER NOT NULL,
    user_id       INTEGER NOT NULL,
    used_at       DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (invitation_id) REFERENCES "invitation_code" (id),
    FOREIGN KEY (user_id) REFERENCES "user" (id)
);
CREATE TABLE IF NOT EXISTS "user"
(
    id                      INTEGER primary key autoincrement,
    username                TEXT unique NOT NULL COLLATE NOCASE,
    password_hash           TEXT        NOT NULL,
    email                   TEXT unique NOT NULL,
    role                    TEXT        NOT NULL DEFAULT 'user',
    created_at              DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at              DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    active                  integer     NOT NULL DEFAULT 1,
    unread_notifications    INTEGER     NOT NULL DEFAULT 0,
    credit_score            INTEGER     NOT NULL DEFAULT 0,
    coins                   INTEGER     NOT NULL DEFAULT 0,
    avatar_timestamp        INTEGER     NOT NULL DEFAULT 0,
    last_access_date        TEXT        NOT NULL DEFAULT CURRENT_DATE, -- 最后登录日期
    access_days             INTEGER     NOT NULL DEFAULT 0,            -- 累计登录天数
    continuous_access_days  INTEGER     NOT NULL DEFAULT 0,            -- 连续登录天数
    last_checkin_date       TEXT        not null DEFAULT CURRENT_DATE, -- 最后打卡日期
    checkin_days            INTEGER     not null default 0,            -- 累计打卡天数
    continuous_checkin_days INTEGER     not null default 0,            -- 连续打卡天数
    bio                     TEXT        NOT NULL DEFAULT '',
    address                 TEXT        NOT NULL DEFAULT '',
    timezone                TEXT        NOT NULL DEFAULT 'UTC',
    language                TEXT        NOT NULL DEFAULT 'en_US',
    public_email            INTEGER     NOT NULL DEFAULT 0,
    totp_secret             TEXT
);
CREATE TABLE user_login_rewards
(
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    date       DATE     NOT NULL DEFAULT CURRENT_DATE,
    user_id    INTEGER  NOT NULL,
    category   TEXT     NOT NULL CHECK (category IN ('login', 'checkin')),
    credit     INTEGER  NOT NULL,
    coins      INTEGER  NOT NULL,
    FOREIGN KEY (user_id) REFERENCES user (id)
);
CREATE TABLE IF NOT EXISTS "topic"
(
    id             INTEGER
        primary key autoincrement,
    user_id        INTEGER                            not null
        references user,
    node_id        INTEGER                            not null
        references node,
    title          TEXT                               not null,
    content        TEXT                               not null,
    view_count     INTEGER  default 0                 not null,
    is_pinned      BOOLEAN  default 0                 not null,
    is_locked      BOOLEAN  default 0                 not null,
    created_at     DATETIME default CURRENT_TIMESTAMP not null,
    updated_at     DATETIME default CURRENT_TIMESTAMP not null,
    reply_count    INTEGER  default 0                 not null,
    last_reply_by  TEXT,
    rank_score     INTEGER  default 0                 not null,
    bot            integer  default 0                 not null,
    content_render text     default ''                not null,
    content_plain  text     default ''                not null
);
CREATE TABLE IF NOT EXISTS "comment"
(
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    article_id     INTEGER             NOT NULL,
    user_id        INTEGER             NOT NULL,
    content        TEXT                NOT NULL,
    content_render text     default '' not null,
    content_plain  text     default '' not null,
    created_at     DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at     DATETIME DEFAULT CURRENT_TIMESTAMP,
    floor          INTEGER             NOT NULL DEFAULT 0,
    bot            integer  default 0  not null,
    FOREIGN KEY (article_id) REFERENCES "topic" (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES "user" (id)
);
CREATE TABLE IF NOT EXISTS "node"
(
    id                        INTEGER primary key autoincrement,
    name                      TEXT unique                        not null,
    slug                      TEXT unique                        not null,
    description               TEXT                               not null,
    created_at                DATETIME default CURRENT_TIMESTAMP not null,
    show_in_list              INTEGER  default 1                 not null,
    background_image          TEXT     default ''                not null,
    icon_image                TEXT     default ''                not null,
    node_color                TEXT     default ''                not null,
    custom_html               text     default ''                not null,
    member_access_required    integer  default 0                 not null,
    moderator_access_required integer  default 0                 not null,
    topic_reward              INTEGER  default 0                 not null,
    comment_reward            INTEGER  default 0                 not null,
    isolated                  integer  default 0                 not null,
    access_only               integer  default 0                 not null,
    topic_count               integer  default 0                 not null
);
CREATE TRIGGER update_node_topic_count_insert
    AFTER INSERT
    ON topic
BEGIN
    UPDATE node
    SET topic_count = (SELECT COUNT(id)
                       FROM topic
                       WHERE node_id = NEW.node_id)
    WHERE id = NEW.node_id;
END;
CREATE TRIGGER update_node_topic_count_delete
    AFTER DELETE
    ON topic
BEGIN
    UPDATE node
    SET topic_count = (SELECT COUNT(id)
                       FROM topic
                       WHERE node_id = OLD.node_id)
    WHERE id = OLD.node_id;
END;
CREATE TRIGGER update_node_topic_count_update
    AFTER UPDATE OF node_id
    ON topic
    WHEN OLD.node_id != NEW.node_id
BEGIN
    -- Update old node count
    UPDATE node
    SET topic_count = (SELECT COUNT(id)
                       FROM topic
                       WHERE node_id = OLD.node_id)
    WHERE id = OLD.node_id;

    -- Update new node count
    UPDATE node
    SET topic_count = (SELECT COUNT(id)
                       FROM topic
                       WHERE node_id = NEW.node_id)
    WHERE id = NEW.node_id;
END;
