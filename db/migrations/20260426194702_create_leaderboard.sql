create table if not exists leaderboard (
    id blob not null primary key, -- UUID
    name text not null,
    clicks integer not null,
    created_at integer not null -- timestamp
);
