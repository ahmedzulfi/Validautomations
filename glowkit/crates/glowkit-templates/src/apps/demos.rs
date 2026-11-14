use glowkit_core::demo::{Category, Demo};

use super::common::run_demo;
use super::*;

fn embedded<A: Default>(f: impl FnOnce(&mut A)) -> A {
    let mut a = A::default();
    f(&mut a);
    a
}

pub fn all() -> Vec<Demo> {
    vec![
    Demo {
        id: "app-analytics",
        name: "Analytics Dashboard",
        category: Category::Templates,
        description: "KPI tiles with sparklines, revenue area chart, traffic donut, top pages and recent orders, with date-range tabs that reload the data behind skeletons.",
        covers: &["shadcn/ui: Dashboard example", "Tremor: Analytics dashboard", "Tailwind UI: Stats + charts dashboard"],
        code: r##"let mut app = AnalyticsApp::default(); // store in your app state
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<AnalyticsApp>(|a| a.scoped_theme = true), AnalyticsApp::show),
    },
    Demo {
        id: "app-crm",
        name: "SaaS Admin / CRM",
        category: Category::Templates,
        description: "Customers table with search, filters, sortable columns, selection and bulk delete, a detail sheet and an add dialog; a drag-and-drop deal pipeline.",
        covers: &["shadcn/ui: Tasks / data table example", "Tailwind UI: CRM", "Twenty: People table", "Attio: Pipeline board"],
        code: r##"let mut app = CrmApp::default();
app.route = "pipeline".into(); // or "customers"
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<CrmApp>(|a| a.scoped_theme = true), CrmApp::show),
    },
    Demo {
        id: "app-crm-pipeline",
        name: "Sales Pipeline Board",
        category: Category::Templates,
        description: "The CRM's deal pipeline: stage columns with counts and totals, deal cards you drag between stages, and toasts when a deal moves or is won.",
        covers: &["Pipedrive: Pipeline", "Attio: Board view", "Tailwind UI: Kanban"],
        code: r##"let mut app = CrmApp::default();
app.route = "pipeline".into();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<CrmApp>(|a| { a.scoped_theme = true; a.route = "pipeline".into(); }), CrmApp::show),
    },
    Demo {
        id: "app-commerce",
        name: "E-commerce Admin",
        category: Category::Templates,
        description: "Store admin with a low-stock alert, KPIs, order tabs and one-click fulfil; a filterable product grid with featured stars; inventory levels with reordering.",
        covers: &["Shopify Polaris: Orders index", "Tailwind UI: Ecommerce admin", "Medusa: Admin dashboard"],
        code: r##"let mut app = CommerceApp::default();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<CommerceApp>(|a| a.scoped_theme = true), CommerceApp::show),
    },
    Demo {
        id: "app-commerce-products",
        name: "Product Catalog Grid",
        category: Category::Templates,
        description: "The store's product grid: search, category chips, sort, grid / list toggle, stock badges, ratings and featured stars, with hover lift.",
        covers: &["Shopify Polaris: Product index", "Tailwind UI: Product lists"],
        code: r##"let mut app = CommerceApp::default();
app.route = "products".into();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<CommerceApp>(|a| { a.scoped_theme = true; a.route = "products".into(); }), CommerceApp::show),
    },
    Demo {
        id: "app-finance",
        name: "Finance Dashboard",
        category: Category::Templates,
        description: "Gradient balance card, account cards with sparklines, cash-flow bars, spending vs. budget, filterable transactions and a working transfer dialog.",
        covers: &["Tailwind UI: Finance dashboard", "Tremor: Financial overview", "Mercury: Accounts home"],
        code: r##"let mut app = FinanceApp::default();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<FinanceApp>(|a| a.scoped_theme = true), FinanceApp::show),
    },
    Demo {
        id: "app-projects",
        name: "Project Management",
        category: Category::Templates,
        description: "Project header with progress and team, collapsible status groups with completable tasks, priorities and due dates; Ctrl+2 flips to a drag-and-drop board.",
        covers: &["Linear: Project issues", "Asana: List + Board", "Tailwind UI: Project list"],
        code: r##"let mut app = ProjectsApp::default();
app.view = 1; // 0 = list, 1 = board
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<ProjectsApp>(|a| a.scoped_theme = true), ProjectsApp::show),
    },
    Demo {
        id: "app-projects-board",
        name: "Task Board",
        category: Category::Templates,
        description: "The project's board view: status columns, task cards with labels, priority flags, sub-task progress and assignees; drag cards between columns.",
        covers: &["Linear: Board", "Trello: Board", "Jira: Kanban"],
        code: r##"let mut app = ProjectsApp::default();
app.view = 1;
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<ProjectsApp>(|a| { a.scoped_theme = true; a.view = 1; }), ProjectsApp::show),
    },
    Demo {
        id: "app-mail",
        name: "Mail Client",
        category: Category::Templates,
        description: "Three panes: folders with live unread counts, a searchable list with All / Unread tabs, a reading pane with actions and inline reply; ↑ ↓ E Del C shortcuts and a compose dialog.",
        covers: &["shadcn/ui: Mail example", "Superhuman: Inbox", "Apple Mail: Three-pane layout"],
        code: r##"let mut app = MailApp::default();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<MailApp>(|a| a.scoped_theme = true), MailApp::show),
    },
    Demo {
        id: "app-chat",
        name: "Team Chat",
        category: Category::Templates,
        description: "Dual sidebar of channels and DMs, a message stream with day divider, grouped messages, toggleable reactions and threads, and a composer where teammates type back.",
        covers: &["Slack: Channel view", "Discord: Text channel", "Tailwind UI: Chat"],
        code: r##"let mut app = ChatApp::default();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<ChatApp>(|a| a.scoped_theme = true), ChatApp::show),
    },
    Demo {
        id: "app-assistant",
        name: "AI Assistant",
        category: Category::Templates,
        description: "Chat history in the sidebar, a conversation with streamed reasoning and Markdown answers, message actions, suggestions, attachments and a model picker in the composer.",
        covers: &["ChatGPT: App layout", "Vercel AI Chatbot template", "shadcn/ui: AI chat"],
        code: r##"let mut app = AssistantApp::default();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<AssistantApp>(|a| a.scoped_theme = true), AssistantApp::show),
    },
    Demo {
        id: "app-settings",
        name: "Settings Pages",
        category: Category::Templates,
        description: "Profile, account, notifications, appearance (a theme picker that installs real themes, radius and density), billing plan cards with yearly pricing, and team roles.",
        covers: &["shadcn/ui: Forms / Settings example", "Tailwind UI: Settings screens", "Vercel: Team settings"],
        code: r##"let mut app = SettingsApp::default();
app.route = "settings/billing".into();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<SettingsApp>(|a| a.scoped_theme = true), SettingsApp::show),
    },
    Demo {
        id: "app-settings-billing",
        name: "Billing & Plans",
        category: Category::Templates,
        description: "The settings billing page: plan cards with a monthly / yearly switch, upgrade and downgrade actions, payment method and downloadable invoices.",
        covers: &["Stripe: Customer portal", "Tailwind UI: Billing settings"],
        code: r##"let mut app = SettingsApp::default();
app.route = "settings/billing".into();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<SettingsApp>(|a| { a.scoped_theme = true; a.route = "settings/billing".into(); }), SettingsApp::show),
    },
    Demo {
        id: "app-files",
        name: "File Manager",
        category: Category::Templates,
        description: "Folder tree with storage meter, breadcrumbs, search, grid / list toggle, double-click to open folders, and a details panel with preview, metadata and sharing.",
        covers: &["Google Drive: My Drive", "Dropbox: Files", "Tailwind UI: File grid"],
        code: r##"let mut app = FileManagerApp::default();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<FileManagerApp>(|a| a.scoped_theme = true), FileManagerApp::show),
    },
    Demo {
        id: "app-music",
        name: "Music Player",
        category: Category::Templates,
        description: "Library sidebar, playlist hero, track list with live equalizer bars and likes, an up-next queue and a now-playing bar with seek, volume, shuffle and repeat (Space, ← →).",
        covers: &["Spotify: Desktop player", "shadcn/ui: Music example", "Apple Music: Playlist view"],
        code: r##"let mut app = MusicApp::default();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<MusicApp>(|a| a.scoped_theme = true), MusicApp::show),
    },
    Demo {
        id: "app-calendar",
        name: "Calendar",
        category: Category::Templates,
        description: "Mini month picker and calendar toggles, Month / Week views (M / W, ← →, T), event chips with +n more, positioned week blocks with a now line, and click-to-create.",
        covers: &["Google Calendar: Week view", "Notion Calendar", "Tailwind UI: Calendars"],
        code: r##"let mut app = CalendarApp::default();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<CalendarApp>(|a| a.scoped_theme = true), CalendarApp::show),
    },
    Demo {
        id: "app-docs",
        name: "Docs / Knowledge Base",
        category: Category::Templates,
        description: "Grouped page tree, an article with breadcrumbs, callouts and code, a scroll-synced table of contents, prev / next cards and a helpful-or-not prompt.",
        covers: &["Nextra: Docs theme", "Mintlify: Docs layout", "shadcn/ui: Docs site"],
        code: r##"let mut app = DocsApp::default();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<DocsApp>(|a| a.scoped_theme = true), DocsApp::show),
    },
    Demo {
        id: "app-devconsole",
        name: "Developer Console",
        category: Category::Templates,
        description: "Usage page with KPI tiles, request and error charts and a 90-day uptime tracker; API keys with a reveal-once create dialog and revoke confirmation; a live log stream.",
        covers: &["Stripe: Developers dashboard", "Vercel: Usage", "Supabase: API settings"],
        code: r##"let mut app = DevConsoleApp::default();
app.route = "keys".into(); // "usage" | "keys" | "logs"
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<DevConsoleApp>(|a| a.scoped_theme = true), DevConsoleApp::show),
    },
    Demo {
        id: "app-devconsole-logs",
        name: "Live Log Stream",
        category: Category::Templates,
        description: "The developer console's log viewer: a pulsing live badge, level toggles, filter, pause / resume (P), auto-follow scrolling and highlighted errors.",
        covers: &["Vercel: Runtime logs", "Datadog: Live tail", "Supabase: Logs explorer"],
        code: r##"let mut app = DevConsoleApp::default();
app.route = "logs".into();
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<DevConsoleApp>(|a| { a.scoped_theme = true; a.route = "logs".into(); }), DevConsoleApp::show),
    },
    Demo {
        id: "app-auth-gate",
        name: "Auth Gate & Sign-in",
        category: Category::Templates,
        description: "Signed-out state of an app: the shell stays, content becomes a sign-in card with OAuth buttons, validation, remember me and a loading sign-in that unlocks the workspace.",
        covers: &["shadcn/ui: Authentication example", "Clerk: Sign-in", "Tailwind UI: Sign-in forms"],
        code: r##"let mut app = AuthGateApp::default(); // signed_in = false
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<AuthGateApp>(|a| a.scoped_theme = true), AuthGateApp::show),
    },
    Demo {
        id: "app-gated-states",
        name: "Gated Empty States",
        category: Category::Templates,
        description: "Signed-in gates: an upgrade wall over a frosted preview (Reports), access denied with request access (Admin) and a first-run empty state that seeds sample data (Projects).",
        covers: &["Linear: Upgrade prompt", "Vercel: Empty states", "Tailwind UI: Empty states"],
        code: r##"let mut app = AuthGateApp::default();
app.signed_in = true;
app.route = "reports".into(); // "reports" | "admin" | "projects"
app.show(ui);"##,
        show: |ui| run_demo(ui, || embedded::<AuthGateApp>(|a| { a.scoped_theme = true; a.signed_in = true; a.route = "reports".into(); }), AuthGateApp::show),
    },
    ]
}
