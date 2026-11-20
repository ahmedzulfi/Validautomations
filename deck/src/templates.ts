/** Starter decks, written as outlines so they double as examples of the outline format. */
export interface Template {
  id: string;
  name: string;
  description: string;
  themeId: string;
  outline: string;
}

export const TEMPLATES: Template[] = [
  {
    id: "pitch",
    name: "Startup pitch",
    description: "Problem, solution, traction, ask",
    themeId: "midnight",
    outline: `# Acme Ledger
Bookkeeping that runs itself for small businesses

## Small businesses lose 120 hours a year to bookkeeping
- Receipts live in inboxes, bank apps and shoeboxes
- Accountants charge for data entry, not advice
- Mistakes surface at tax time, when they're expensive

## Acme reconciles every transaction automatically
<!-- layout: image -->
Connect your bank once; Acme categorizes, matches receipts and flags anything odd.
![A clean dashboard on a laptop in a sunlit café]()

## Traction since launch
- **4,200** paying businesses
- **$1.9M** annual recurring revenue
- **3%** monthly churn

## Revenue is compounding
\`\`\`chart line
unit: $k
Jan: 40
Mar: 62
May: 95
Jul: 128
Sep: 160
\`\`\`

## Why we win
### Legacy tools
- Manual categorization
- Monthly close takes days
- Priced per accountant hour
### Acme
- 96% auto-categorized
- Books closed daily
- Flat $29 per month

---

> "I got my Sundays back. Acme does in seconds what took me all weekend."
— Priya N., café owner

## Roadmap
- Q1 2027: Payroll integration
- Q2 2027: Multi-entity support
- Q3 2027: Tax filing in 3 states
- Q4 2027: Open API for accountants

## Raising $6M to reach 25,000 businesses
<!-- layout: closing -->
hello@acmeledger.example
`,
  },
  {
    id: "launch",
    name: "Product launch",
    description: "What's new, why it matters, how to get it",
    themeId: "studio",
    outline: `# Introducing Focus Mode
Ship the work that matters, without the noise

## Teams switch context 300 times a day
- Every ping costs about 20 minutes of deep work
- Most notifications aren't urgent
- Calendars fill up before work starts

## What's in Focus Mode
### For individuals
- One-click focus blocks
- Smart notification batching
- Daily focus score
### For teams
- Shared quiet hours
- Meeting-free days
- Focus insights for leads

## Beta results
- **2.4h** more deep work per day
- **38%** fewer interruptions
- **91%** would keep it on

## Availability
| Plan | Focus Mode | Team insights |
| --- | --- | --- |
| Free | Yes | No |
| Pro | Yes | Yes |
| Enterprise | Yes | Yes, with exports |

## Thank you
Available today in every workspace
`,
  },
  {
    id: "qbr",
    name: "Quarterly review",
    description: "Results, learnings and next quarter's plan",
    themeId: "paper",
    outline: `# Q3 Business Review
Growth team · October 2026

## We beat the revenue target by 12%
- **$4.8M** revenue, target $4.3M
- **+18%** new customers quarter over quarter
- **94%** gross retention

## Revenue by month
\`\`\`chart bar
unit: $M
Jul: 1.4
Aug: 1.6
Sep: 1.8
\`\`\`

## Where new customers came from
\`\`\`chart donut
Referrals: 42
Organic search: 31
Partners: 17
Paid: 10
\`\`\`

## What worked and what didn't
### Worked
- Referral credits doubled invites
- Onboarding checklist cut time-to-value
### Didn't
- Paid social stayed expensive
- Webinar attendance fell in August

## Q4 plan
- Oct: Launch partner portal
- Nov: Annual plan promotion
- Dec: Self-serve enterprise trial

## Questions?
`,
  },
  {
    id: "lesson",
    name: "Lesson / workshop",
    description: "Teach one idea clearly",
    themeId: "forest",
    outline: `# How Photosynthesis Works
Biology 101 · Week 4

## Plants make food from light
- Light energy is captured by chlorophyll
- Water and carbon dioxide are the raw materials
- Glucose and oxygen are the products

## The two stages
### Light reactions
- Happen in the thylakoids
- Split water, release oxygen
- Produce ATP and NADPH
### Calvin cycle
- Happens in the stroma
- Fixes carbon dioxide
- Builds glucose

---

> "Nature does nothing uselessly."
— Aristotle

## Check your understanding
- What goes in, and what comes out?
- Where does the oxygen come from?
- Why do leaves look green?

## Thanks for learning with us
Next week: cellular respiration
`,
  },
];
