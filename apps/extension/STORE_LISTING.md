# Store listing

Everything the Chrome Web Store (and Firefox Add-ons) asks for, ready to paste. Build the upload with
`pnpm --filter extension zip` (Chrome) or `pnpm --filter extension zip:firefox`; the zip lands in
`apps/extension/.output/`.

## Name

Stash

## Short description (132 characters max)

Your saves from X, LinkedIn, Reddit, YouTube, Instagram and your browser in one library, sorted into categories by AI.

## Category

Productivity

## Detailed description

Stash brings everything you saved into one library and sorts it for you.

Connect the extension to your Stash account, press Sync, and it gathers:
- X bookmarks
- LinkedIn saved posts
- Reddit saved posts and comments
- YouTube Watch later and Liked videos
- Instagram saved posts
- Your browser bookmarks, including new ones as you make them

In the Stash web app, the AI suggests categories that fit what you actually save. You approve or edit them, and it sorts everything. Saves it is unsure about wait in an inbox. Search by words or by meaning, ask a question and get an answer from your own saves, get a weekly digest, and find broken links.

How it works, and what it does not do:
- It only acts when you press Sync, or once a day if you turn that on.
- It opens your own saved lists while you are logged in and reads the data the page already loads, scrolling at a person's pace. For Reddit and Instagram it reads your saved list with your login.
- It never posts, likes, unsaves or changes anything, and never sees your passwords.
- It sends what it reads only to your Stash server, using a token you create.
- "Save this page" adds the current tab to your library.

## Single purpose (Chrome asks for this)

Collect the user's own saved posts and bookmarks from the sites they use and send them to their Stash library, where they are organised.

## Permission justifications

| Permission | Why |
|---|---|
| `bookmarks` | Reads the browser's bookmarks to add them to the library, and adds new bookmarks as they are made. Never changes or deletes bookmarks. |
| `storage` | Keeps the server address, the access token and each source's last sync status in the browser. |
| `alarms` | Runs the optional once-a-day sync. |
| `tabs` | Opens the saved-lists page on X, LinkedIn or YouTube to sync it, closes it afterwards, and reads the current tab's address for "Save this page". |
| Host `x.com` | Reads the user's own bookmarks page. |
| Host `www.linkedin.com` | Reads the user's own saved-posts page. |
| Host `www.youtube.com` | Reads the user's own Watch later and Liked videos playlists. |
| Host `www.reddit.com` | Reads the user's own saved listing with their login. |
| Host `www.instagram.com` | Reads the user's own saved posts with their login. |

Remote code: none. All code ships in the package.

## Data use disclosures (Chrome privacy practices tab)

Data collected: website content (the text, author and link of saved posts) and bookmarks. Also authentication information: the Stash access token the user pastes in, kept locally.

- Sent only to the Stash server the user connects (their own account), over HTTPS.
- Not sold to third parties.
- Not used or transferred for purposes unrelated to the single purpose.
- Not used to determine creditworthiness or for lending.

Privacy policy URL: `https://YOUR-STASH-DOMAIN/privacy` (the web app serves it).

## Screenshots (1280×800)

1. The popup after a sync, showing each source.
2. The library sorted into categories.
3. Reviewing the suggested categories.
4. Ask your saves, with an answer and the saves it used.

## Firefox Add-ons

`pnpm --filter extension zip:firefox` makes the upload, plus a sources zip AMO may ask for. Before
submitting, change `browser_specific_settings.gecko.id` in `wxt.config.ts` to an id you own. Requires
Firefox 128 or later (for page-world content scripts).

## Safari

Safari runs the same extension after conversion on a Mac with Xcode:

```bash
pnpm --filter extension build
xcrun safari-web-extension-converter apps/extension/.output/chrome-mv3 --app-name Stash
```

Then build the generated Xcode project and enable the extension in Safari's settings. Distribution
needs an Apple Developer account.
