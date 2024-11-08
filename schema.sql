CREATE TABLE IF NOT EXISTS property (
    uuid UUID PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    secret UUID NOT NULL
);

CREATE TABLE IF NOT EXISTS analytics (
    property UUID NOT NULL,
    date TIMESTAMP NOT NULL,
    peer_addr INET,
    user_agent TEXT,
    referer TEXT,
    geolocation JSON,
    device JSON,
    method TEXT NOT NULL,
    uri TEXT NOT NULL,
    UNIQUE (peer_addr, user_agent, method, uri)
);
CREATE INDEX date ON analytics(date);
CREATE UNIQUE INDEX raw_info ON analytics(peer_addr, user_agent, method, uri);

CREATE TABLE IF NOT EXISTS newsletter_subscriber (
    email TEXT PRIMARY KEY,
    subscribe_date TIMESTAMP NOT NULL,
    unsubscribe_date TIMESTAMP,
    unsubscribe_token UUID,
    UNIQUE (email)
);
