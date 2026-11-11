// Runs in the page's own JavaScript world. Watches the responses the site's web app already fetches and
// copies the matching ones to the extension with window.postMessage. It never makes a request itself
// and never changes a response.

export const STASH_MESSAGE = '__stash_capture__';

export type CaptureMessage = {[STASH_MESSAGE]: true; tag: string; url: string; data: unknown};

export function installInterceptor(tag: string, matches: (url: string) => boolean): void {
  const w = window as unknown as {__stashIntercepting?: boolean};
  if (w.__stashIntercepting) return;
  w.__stashIntercepting = true;

  const post = (url: string, data: unknown) => {
    const message: CaptureMessage = {[STASH_MESSAGE]: true, tag, url, data};
    window.postMessage(message, location.origin);
  };

  const originalFetch = window.fetch;
  window.fetch = async function stashFetch(this: unknown, ...args: Parameters<typeof fetch>) {
    const response = await originalFetch.apply(this as typeof window, args);
    try {
      const input = args[0];
      const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
      if (matches(url)) {
        response.clone().json().then((data) => post(url, data), () => {});
      }
    } catch {
      // Never break the page.
    }
    return response;
  };

  const open = XMLHttpRequest.prototype.open;
  const send = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.open = function stashOpen(this: XMLHttpRequest & {__stashUrl?: string}, ...args: unknown[]) {
    this.__stashUrl = String(args[1]);
    return (open as (...a: unknown[]) => void).apply(this, args);
  };
  XMLHttpRequest.prototype.send = function stashSend(this: XMLHttpRequest & {__stashUrl?: string}, body?: Document | XMLHttpRequestBodyInit | null) {
    const url = this.__stashUrl;
    if (url && matches(url)) {
      this.addEventListener('load', () => {
        try {
          const data = this.responseType === 'json' ? this.response : this.responseType === '' || this.responseType === 'text' ? JSON.parse(this.responseText) : null;
          if (data) post(url, data);
        } catch {
          // Not JSON; ignore.
        }
      });
    }
    return send.call(this, body);
  };
}
