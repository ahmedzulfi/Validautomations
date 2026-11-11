create extension if not exists pgcrypto;

create table users (
  id uuid primary key default gen_random_uuid(),
  email text not null,
  password_hash text not null,
  created_at timestamptz not null default now()
);
create unique index users_email_key on users (lower(email));

create table sessions (
  token_hash text primary key,
  user_id uuid not null references users(id) on delete cascade,
  created_at timestamptz not null default now(),
  expires_at timestamptz not null
);
create index sessions_user_idx on sessions (user_id);

-- Tokens the browser extension uses to send saves to this account.
create table api_tokens (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  name text not null,
  token_hash text not null unique,
  token_hint text not null,
  created_at timestamptz not null default now(),
  last_used_at timestamptz
);
create index api_tokens_user_idx on api_tokens (user_id);

create table categories (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  name text not null,
  description text not null default '',
  sort_order int not null default 0,
  is_ai_generated boolean not null default false,
  created_at timestamptz not null default now()
);
create unique index categories_user_name_key on categories (user_id, lower(name));

create table items (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  -- Every source this was saved from (the same link can be a bookmark and an X post).
  sources text[] not null,
  source_ids text[] not null default '{}',
  canonical_url text not null,
  url text not null,
  title text,
  body text,
  author_name text,
  author_handle text,
  author_avatar text,
  media jsonb,
  links jsonb,
  folder_path text[],
  saved_at timestamptz,
  posted_at timestamptz,
  imported_at timestamptz not null default now(),
  category_id uuid references categories(id) on delete set null,
  category_confidence real,
  -- 'ai' or 'user'. A choice the person made is never overwritten by the AI.
  category_assigned_by text check (category_assigned_by in ('ai', 'user')),
  category_assigned_at timestamptz,
  tags text[] not null default '{}',
  summary text,
  classified_at timestamptz,
  search tsvector generated always as (
    setweight(to_tsvector('simple', coalesce(title, '')), 'A') ||
    setweight(to_tsvector('simple', coalesce(author_name, '') || ' ' || coalesce(author_handle, '')), 'B') ||
    setweight(to_tsvector('simple', coalesce(summary, '')), 'B') ||
    setweight(to_tsvector('simple', coalesce(body, '')), 'C') ||
    setweight(to_tsvector('simple', coalesce(canonical_url, '')), 'D')
  ) stored
);
create unique index items_user_url_key on items (user_id, canonical_url);
create index items_user_time_idx on items (user_id, coalesce(saved_at, imported_at) desc);
create index items_user_category_idx on items (user_id, category_id);
create index items_unclassified_idx on items (user_id) where classified_at is null;
create index items_search_idx on items using gin (search);
create index items_tags_idx on items using gin (tags);

create table sync_runs (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  source text not null,
  items_seen int not null default 0,
  items_added int not null default 0,
  error text,
  finished_at timestamptz not null default now()
);
create index sync_runs_user_idx on sync_runs (user_id, finished_at desc);

-- A set of categories the AI proposed, waiting for the person to approve or edit.
create table taxonomy_proposals (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  categories jsonb not null,
  status text not null default 'pending' check (status in ('pending', 'applied', 'dismissed')),
  created_at timestamptz not null default now()
);
