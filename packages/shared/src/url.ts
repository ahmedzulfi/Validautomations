const TRACKING_PARAMS = /^(utm_[a-z]+|fbclid|gclid|mc_cid|mc_eid|igshid|ref_src|ref_url|si|trk|trackingId|lipi|rcm)$/i;
const X_HOSTS = new Set(['twitter.com', 'mobile.twitter.com', 'x.com', 'mobile.x.com']);

/**
 * The form of a URL used to spot the same thing saved twice: lower-case host without www, X and
 * Twitter as one host, tracking parameters and the fragment dropped, no trailing slash. A post on X is
 * reduced to its id, since the handle in the path changes when the author renames.
 */
export function canonicalUrl(input: string): string {
  let u: URL;
  try {
    u = new URL(input.trim());
  } catch {
    return input.trim();
  }
  u.hash = '';
  u.hostname = u.hostname.toLowerCase().replace(/^www\./, '');
  if (u.protocol === 'http:') u.protocol = 'https:';
  if (X_HOSTS.has(u.hostname)) {
    u.hostname = 'x.com';
    const m = u.pathname.match(/\/status(?:es)?\/(\d+)/);
    if (m) return `https://x.com/i/status/${m[1]}`;
  }
  const keep = [...u.searchParams.entries()].filter(([k]) => !TRACKING_PARAMS.test(k));
  keep.sort(([a], [b]) => a.localeCompare(b));
  u.search = keep.length ? '?' + new URLSearchParams(keep).toString() : '';
  if (u.pathname.length > 1 && u.pathname.endsWith('/')) u.pathname = u.pathname.replace(/\/+$/, '');
  const out = u.toString();
  return u.pathname === '/' && !u.search ? out.replace(/\/$/, '') : out;
}

export function hostOf(input: string): string {
  try {
    return new URL(input).hostname.replace(/^www\./, '');
  } catch {
    return '';
  }
}
