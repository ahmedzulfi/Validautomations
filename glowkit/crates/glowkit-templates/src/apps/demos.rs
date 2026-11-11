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
    ]
}
