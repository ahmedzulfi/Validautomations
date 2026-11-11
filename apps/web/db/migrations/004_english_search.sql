-- Search by words matches word forms (index, indexes, indexing) by using English stemming.
drop index if exists items_search_idx;
alter table items drop column search;
alter table items add column search tsvector generated always as (
  setweight(to_tsvector('english', coalesce(title, '')), 'A') ||
  setweight(to_tsvector('simple', coalesce(author_name, '') || ' ' || coalesce(author_handle, '')), 'B') ||
  setweight(to_tsvector('english', coalesce(summary, '')), 'B') ||
  setweight(to_tsvector('english', coalesce(body, '')), 'C') ||
  setweight(to_tsvector('simple', coalesce(canonical_url, '')), 'D')
) stored;
create index items_search_idx on items using gin (search);
