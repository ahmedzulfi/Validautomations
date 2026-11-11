import {useEffect, useState} from 'react';
import {Button} from '@/components/lime/ui/button';
import {Field, FieldDescription, FieldLabel} from '@/components/lime/ui/field';
import {Input} from '@/components/lime/ui/input';
import {List, ListItem} from '@/components/lime/ui/list';
import {Status} from '@/components/lime/ui/status';
import {Switch} from '@/components/lime/ui/switch';
import {useToast} from '@/components/lime/ui/toast';
import {fetchMe} from '../../utils/api';
import type {SavePageMessage, SavePageReply, SyncMessage, SyncSource} from '../../utils/messages';
import {
  emptyStatus, getAllStatus, getSettings, normalizeServerUrl, saveSettings, SOURCE_NAMES, type Settings, type SourceStatus,
} from '../../utils/settings';

const SOURCES: SyncSource[] = ['x', 'linkedin', 'bookmark'];

export function App() {
  const [settings, setSettings] = useState<Settings | null | undefined>(undefined);
  useEffect(() => {
    void getSettings().then(setSettings);
  }, []);
  if (settings === undefined) return null;
  return (
    <main className="flex w-[360px] flex-col gap-5 bg-page p-4 text-fg">
      {settings ? <Connected settings={settings} onChange={setSettings}/> : <Connect onConnected={setSettings}/>}
    </main>
  );
}

function Connect({onConnected}: {onConnected: (s: Settings) => void}) {
  const [server, setServer] = useState('');
  const [token, setToken] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const connect = async () => {
    setError(null);
    const serverUrl = normalizeServerUrl(server);
    if (!serverUrl) return setError('Enter the server address shown in Stash settings.');
    if (!token.trim().startsWith('stash_')) return setError('Paste the token from Stash settings. It starts with stash_.');
    setPending(true);
    try {
      // The server's API answers cross-origin requests and is reached with the token, so no extra
      // host permission is needed for an address the person chooses.
      const me = await fetchMe({serverUrl, token: token.trim()});
      const next: Settings = {serverUrl, token: token.trim(), email: me.email, autoSync: false};
      await saveSettings(next);
      onConnected(next);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not connect.');
    } finally {
      setPending(false);
    }
  };

  return (
    <>
      <div className="flex flex-col gap-1">
        <h1 className="text-xl leading-tight font-semibold tracking-tight">Connect to Stash</h1>
        <p className="text-sm text-fg-2">In Stash, open Settings, create a token, and paste it here with the server address.</p>
      </div>
      <form className="flex flex-col gap-4" onSubmit={(e) => { e.preventDefault(); void connect(); }}>
        <Field name="server">
          <FieldLabel>Server address</FieldLabel>
          <Input value={server} onChange={(e) => setServer(e.target.value)} placeholder="https://stash.example.com" autoFocus/>
        </Field>
        <Field name="token">
          <FieldLabel>Token</FieldLabel>
          <Input value={token} onChange={(e) => setToken(e.target.value)} type="password" autoComplete="off"/>
          <FieldDescription>Kept only in this browser.</FieldDescription>
        </Field>
        {error && <p role="alert" className="text-sm text-danger-text">{error}</p>}
        <Button type="submit" size="lg" className="w-full" pending={pending}>Connect</Button>
      </form>
    </>
  );
}

function Connected({settings, onChange}: {settings: Settings; onChange: (s: Settings | null) => void}) {
  const toast = useToast();
  const [status, setStatus] = useState<Record<SyncSource, SourceStatus>>({x: emptyStatus(), linkedin: emptyStatus(), bookmark: emptyStatus()});
  const [saving, setSaving] = useState(false);
  const anyRunning = SOURCES.some((s) => status[s].state === 'running');

  useEffect(() => {
    void getAllStatus().then(setStatus);
    const listener = () => void getAllStatus().then(setStatus);
    browser.storage.onChanged.addListener(listener);
    return () => browser.storage.onChanged.removeListener(listener);
  }, []);

  const sync = (source: SyncSource | 'all') => {
    const message: SyncMessage = {type: 'stash:sync', source};
    void browser.runtime.sendMessage(message);
  };

  const savePage = async () => {
    setSaving(true);
    const [tab] = await browser.tabs.query({active: true, currentWindow: true});
    const message: SavePageMessage = {type: 'stash:save-page', url: tab?.url ?? '', title: tab?.title ?? ''};
    const reply = (await browser.runtime.sendMessage(message)) as SavePageReply;
    setSaving(false);
    toast.show({title: reply.ok ? (reply.added ? 'Saved to your library' : 'Already in your library') : reply.error});
  };

  const setAutoSync = async (autoSync: boolean) => {
    const next = {...settings, autoSync};
    await saveSettings(next);
    onChange(next);
  };

  return (
    <>
      <div className="flex items-center justify-between gap-2">
        <div className="flex min-w-0 flex-col">
          <h1 className="text-xl leading-tight font-semibold tracking-tight">Stash</h1>
          <p className="truncate text-sm text-fg-2">{settings.email}</p>
        </div>
        <Button variant="ghost" size="sm" onClick={() => void browser.tabs.create({url: `${settings.serverUrl}/library`})}>Open library</Button>
      </div>

      <List variant="grouped" aria-label="Sources">
        {SOURCES.map((source) => (
          <ListItem
            key={source}
            title={SOURCE_NAMES[source]}
            description={describe(status[source])}
            trailing={status[source].state === 'running'
              ? <Status status="running" label="Syncing"/>
              : <Button variant="outline" size="sm" disabled={anyRunning} onClick={() => sync(source)}>Sync</Button>}
          />
        ))}
      </List>

      <div className="flex flex-col gap-2">
        <Button size="lg" className="w-full" pending={anyRunning} onClick={() => sync('all')}>Sync everything</Button>
        <Button variant="secondary" className="w-full" pending={saving} onClick={() => void savePage()}>Save this page</Button>
      </div>

      <label className="flex items-center justify-between gap-3 text-sm text-fg">
        Sync once a day
        <Switch checked={settings.autoSync} onCheckedChange={(v: boolean) => void setAutoSync(v)}/>
      </label>

      <p className="text-sm text-fg-2">
        X and LinkedIn sync by opening your saved list and scrolling it, as you would. Stay logged in to them in this browser.
      </p>

      <Button variant="link" className="self-start text-sm" onClick={async () => { await saveSettings(null); onChange(null); }}>
        Disconnect
      </Button>
    </>
  );
}

function describe(s: SourceStatus): string {
  if (s.state === 'running') return s.found ? `Found ${s.found} so far` : 'Starting';
  if (s.state === 'failed') return s.message ?? 'Sync failed';
  if (s.state === 'done') {
    const when = new Date(s.updatedAt).toLocaleString(undefined, {month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit'});
    return `${s.added} new of ${s.found}, ${when}`;
  }
  return 'Not synced yet';
}
