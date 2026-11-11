-- When the AI is sorting this person's saves in the background, until when (cleared when it finishes).
alter table users add column sorting_until timestamptz;
alter table users add column sorting_error text;
