import {emailConfigured, sendDueDigests} from '@/lib/server/digest';

export const runtime = 'nodejs';
export const maxDuration = 300;

/**
 * Sends the weekly digests. Vercel Cron calls this on the schedule in vercel.json and sends
 * CRON_SECRET as a Bearer token; anywhere else, call it the same way from your own scheduler.
 */
export async function GET(request: Request) {
  const secret = process.env.CRON_SECRET;
  if (!secret || request.headers.get('authorization') !== `Bearer ${secret}`) {
    return Response.json({error: 'Not allowed.'}, {status: 401});
  }
  if (!emailConfigured()) return Response.json({error: 'Set RESEND_API_KEY and STASH_EMAIL_FROM to send digests.'}, {status: 503});
  return Response.json(await sendDueDigests());
}
