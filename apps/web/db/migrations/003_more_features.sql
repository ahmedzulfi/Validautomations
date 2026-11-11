-- Search by meaning: one embedding per item (pgvector; Supabase and Neon include it).
create extension if not exists vector;
alter table items add column embedding vector(1536);
alter table items add column embedded_at timestamptz;
create index items_embedding_idx on items using hnsw (embedding vector_cosine_ops);
create index items_unembedded_idx on items (user_id) where embedded_at is null;

-- Broken-link checker.
alter table items add column link_status text check (link_status in ('ok', 'broken'));
alter table items add column link_checked_at timestamptz;
alter table users add column link_check_until timestamptz;

-- Weekly digest by email.
alter table users add column digest_enabled boolean not null default false;
alter table users add column digest_last_sent_at timestamptz;
