'use client';

import {useState, useTransition} from 'react';
import {askAction, type AskResult} from '@/app/actions';
import {Message} from '@/components/lime/ai/message';
import {Button} from '@/components/lime/ui/button';
import {Field, FieldLabel} from '@/components/lime/ui/field';
import {Textarea} from '@/components/lime/ui/textarea';
import {ItemCard} from './item-card';

type Props = {
  aiReady: boolean;
  coverage: {embedded: number; total: number};
  categories: Array<{id: string; name: string}>;
};

const EXAMPLES = ['What have I saved about pricing pages?', 'Which posts talk about hiring engineers?', 'Find the Postgres tips I saved'];

export function AskView({aiReady, coverage, categories}: Props) {
  const [question, setQuestion] = useState('');
  const [asked, setAsked] = useState<string | null>(null);
  const [result, setResult] = useState<AskResult | null>(null);
  const [pending, startTransition] = useTransition();

  const ask = (q: string) => {
    const text = q.trim();
    if (!text) return;
    setAsked(text);
    setResult(null);
    startTransition(async () => setResult(await askAction(text)));
  };

  if (!aiReady) {
    return <p className="rounded-panel bg-surface p-4 text-base text-fg-2">Asking needs OPENROUTER_API_KEY on the server.</p>;
  }

  return (
    <div className="flex flex-col gap-6">
      {coverage.total > 0 && coverage.embedded < coverage.total && (
        <p className="text-sm text-fg-2">
          {coverage.embedded.toLocaleString('en-US')} of {coverage.total.toLocaleString('en-US')} saves are searchable so far. The rest are being prepared in the background.
        </p>
      )}

      <form
        className="flex flex-col gap-3"
        onSubmit={(e) => {
          e.preventDefault();
          ask(question);
        }}
      >
        <Field name="question">
          <FieldLabel>Your question</FieldLabel>
          <Textarea
            value={question}
            onChange={(e) => setQuestion(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                ask(question);
              }
            }}
            maxLength={500}
            autoGrow
            placeholder="What have I saved about…"
          />
        </Field>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="submit" pending={pending} disabled={!question.trim()}>Ask</Button>
          {!asked && EXAMPLES.map((ex) => (
            <Button key={ex} type="button" variant="ghost" size="sm" onClick={() => {
              setQuestion(ex);
              ask(ex);
            }}>
              {ex}
            </Button>
          ))}
        </div>
      </form>

      {asked && (
        <section aria-live="polite" className="flex flex-col gap-4">
          <Message from="user" name="You">{asked}</Message>
          {pending && <p className="text-base text-fg-2">Reading your saves…</p>}
          {result && 'error' in result && <p role="alert" className="text-sm text-danger-text">{result.error}</p>}
          {result && 'answer' in result && (
            <>
              <Message name="Stash">
                <p className="whitespace-pre-line">{result.answer}</p>
              </Message>
              {result.sources.length > 0 && (
                <div className="flex flex-col gap-3">
                  <h2 className="text-lg font-semibold">Saves used</h2>
                  <ol className="grid grid-cols-1 gap-4 sm:grid-cols-2">
                    {result.sources.map((item, i) => (
                      <li key={item.id} className="flex">
                        <ItemCard item={item} categories={categories} note={`[${i + 1}]`}/>
                      </li>
                    ))}
                  </ol>
                </div>
              )}
            </>
          )}
        </section>
      )}
    </div>
  );
}
