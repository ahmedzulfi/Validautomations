use super::*;
use egui::{vec2, Align, Layout, RichText, Ui};
use glowkit_core::demo::{Category::Inputs as C, Demo};
use glowkit_core::Theme;

/// Run `f` with persistent demo state stored under `key`.
pub(crate) fn with<T: Clone + Send + Sync + 'static>(
    ui: &mut Ui,
    key: &str,
    init: impl FnOnce() -> T,
    f: impl FnOnce(&mut Ui, &mut T),
) {
    let id = ui.scope_id().with(key);
    let mut v = glowkit_core::state(ui, id, init);
    f(ui, &mut v);
    glowkit_core::set_state(ui, id, v);
}

/// Small uppercase caption above a demo group.
pub(crate) fn caption(ui: &mut Ui, text: &str) {
    let t = Theme::of(ui);
    ui.label(RichText::new(text.to_uppercase()).size(10.5).color(t.muted_foreground).strong());
}

/// A fixed-width column.
pub(crate) fn col(ui: &mut Ui, w: f32, f: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(vec2(w, 0.0), Layout::top_down(Align::Min), |ui| {
        ui.set_width(w);
        f(ui)
    });
}

/// Center a block of known width horizontally.
pub(crate) fn center(ui: &mut Ui, w: f32, f: impl FnOnce(&mut Ui)) {
    let pad = ((ui.available_width() - w) / 2.0).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        col(ui, w, f);
    });
}

/// A card-like panel for composing demos.
pub(crate) fn panel(ui: &mut Ui, w: f32, f: impl FnOnce(&mut Ui)) {
    let t = Theme::of(ui);
    egui::Frame::new()
        .fill(t.card)
        .stroke(t.border_stroke())
        .corner_radius(t.corner())
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            ui.set_width(w - 40.0);
            ui.vertical(f);
        });
}

pub(crate) fn title(ui: &mut Ui, title: &str, desc: &str) {
    let t = Theme::of(ui);
    ui.label(RichText::new(title).size(17.0).strong().color(t.foreground));
    ui.add_space(-4.0);
    ui.label(RichText::new(desc).size(13.0).color(t.muted_foreground));
}

const TODAY: CalendarDate = CalendarDate { year: 2026, month: 11, day: 11 };

pub fn all() -> Vec<Demo> {
    vec![
        Demo {
            id: "button",
            name: "Button",
            category: C,
            description: "Six variants, three sizes, icons, icon-only and loading states, with a springy press.",
            covers: &["shadcn/ui: Button", "Origin UI: Buttons", "Animate UI: Button"],
            code: r##"ui.add(Button::new("Save changes"));
ui.add(Button::new("Secondary").variant(ButtonVariant::Secondary));
ui.add(Button::new("Delete").variant(ButtonVariant::Destructive).icon(UiIcon::Trash));
ui.add(Button::new("Continue").variant(ButtonVariant::Outline).trailing_icon(UiIcon::ArrowRight));
ui.add(Button::new("Please wait").loading(true));
ui.add(Button::icon_only(UiIcon::Settings).variant(ButtonVariant::Outline));"##,
            show: |ui| {
                ui.add_space(20.0);
                center(ui, 640.0, |ui| {
                    ui.spacing_mut().item_spacing = vec2(10.0, 12.0);
                    caption(ui, "Variants");
                    ui.horizontal(|ui| {
                        ui.add(Button::new("Default"));
                        ui.add(Button::new("Secondary").variant(ButtonVariant::Secondary));
                        ui.add(Button::new("Outline").variant(ButtonVariant::Outline));
                        ui.add(Button::new("Ghost").variant(ButtonVariant::Ghost));
                        ui.add(Button::new("Destructive").variant(ButtonVariant::Destructive));
                        ui.add(Button::new("Link").variant(ButtonVariant::Link));
                    });
                    ui.add_space(8.0);
                    caption(ui, "Sizes & icons");
                    ui.horizontal(|ui| {
                        ui.set_min_height(40.0);
                        ui.add(Button::new("Small").small().variant(ButtonVariant::Outline));
                        ui.add(Button::new("Medium").variant(ButtonVariant::Outline));
                        ui.add(Button::new("Large").large().variant(ButtonVariant::Outline));
                        ui.add(Button::new("New project").icon(UiIcon::Plus));
                        ui.add(Button::new("Continue").variant(ButtonVariant::Secondary).trailing_icon(UiIcon::ArrowRight));
                    });
                    ui.add_space(8.0);
                    caption(ui, "States");
                    ui.horizontal(|ui| {
                        with(ui, "loading", || -10.0f64, |ui, since| {
                            let now = ui.input(|i| i.time);
                            let loading = now - *since < 2.0;
                            if ui.add(Button::new(if loading { "Saving…" } else { "Click me" }).loading(loading).icon(UiIcon::Download).width(130.0)).clicked() {
                                *since = now;
                            }
                        });
                        ui.add(Button::new("Please wait").loading(true).variant(ButtonVariant::Outline));
                        ui.add(Button::new("Disabled").disabled(true));
                        ui.add(Button::icon_only(UiIcon::Settings).variant(ButtonVariant::Outline));
                        ui.add(Button::icon_only(UiIcon::Heart).variant(ButtonVariant::Secondary));
                        ui.add(Button::icon_only(UiIcon::Trash).variant(ButtonVariant::Destructive));
                    });
                });
            },
        },
        Demo {
            id: "segmented-control",
            name: "Segmented Control",
            category: C,
            description: "A muted track with a raised thumb that springs between options.",
            covers: &["Animate UI: Tabs (segmented)", "Origin UI: Segmented control", "iOS: Segmented Control"],
            code: r##"ui.add(SegmentedControl::new(&mut view, ["Day", "Week", "Month", "Year"]));"##,
            show: |ui| {
                ui.add_space(80.0);
                with(ui, "seg", || (1usize, 0usize, 2usize), |ui, (a, b, c)| {
                    center(ui, 380.0, |ui| {
                        ui.spacing_mut().item_spacing.y = 20.0;
                        ui.add(SegmentedControl::new(a, ["Day", "Week", "Month", "Year"]).width(380.0));
                        ui.add(SegmentedControl::new(b, [ToggleItem::new(UiIcon::List, "List"), ToggleItem::new(UiIcon::Grid, "Board"), ToggleItem::new(UiIcon::Calendar, "Calendar")]).width(380.0));
                        ui.add(SegmentedControl::new(c, ["Light", "Dark", "System"]).size(ControlSize::Sm).equal_widths(false));
                    });
                });
            },
        },
        Demo {
            id: "input",
            name: "Input",
            category: C,
            description: "Text fields with labels, icons, prefixes, clear buttons, keyboard hints, errors and an animated focus ring.",
            covers: &["shadcn/ui: Input", "Origin UI: Inputs", "shadcn/ui: Input Group", "Animate UI: Input"],
            code: r##"ui.add(Input::new(&mut email).label("Email").placeholder("you@example.com")
    .icon(UiIcon::Mail).clearable(true));
ui.add(Input::new(&mut site).label("Website").prefix("https://").suffix(".com"));
ui.add(Input::new(&mut user).label("Username").required(true)
    .error("This username is already taken."));"##,
            show: |ui| {
                ui.add_space(10.0);
                with(ui, "inp", || ("ada@lovelace.dev".to_string(), "glowkit".to_string(), "admin".to_string(), String::new(), "12.00".to_string()), |ui, (email, site, user, empty, price)| {
                    center(ui, 660.0, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing = vec2(20.0, 16.0);
                            col(ui, 320.0, |ui| {
                                ui.spacing_mut().item_spacing.y = 16.0;
                                ui.add(Input::new(email).label("Email").placeholder("you@example.com").icon(UiIcon::Mail).clearable(true).width(320.0));
                                ui.add(Input::new(site).label("Website").prefix("https://").suffix(".com").width(320.0));
                                ui.add(Input::new(empty).label("Disabled").placeholder("Not editable").disabled(true).width(320.0));
                            });
                            col(ui, 320.0, |ui| {
                                ui.spacing_mut().item_spacing.y = 16.0;
                                ui.add(Input::new(user).label("Username").required(true).icon(UiIcon::User).error("This username is already taken.").width(320.0));
                                ui.add(Input::new(price).label("Price").prefix("$").suffix("USD").description("Charged monthly, cancel anytime.").width(320.0));
                                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                    ui.spacing_mut().item_spacing.x = 10.0;
                                    ui.add(Input::new(&mut String::new()).placeholder("Small").size(ControlSize::Sm).width(100.0).id_salt("s"));
                                    ui.add(Input::new(&mut String::new()).placeholder("Medium").width(100.0).id_salt("m"));
                                    ui.add(Input::new(&mut String::new()).placeholder("Large").size(ControlSize::Lg).width(100.0).id_salt("l"));
                                });
                            });
                        });
                    });
                });
            },
        },
        Demo {
            id: "password-input",
            name: "Password Input",
            category: C,
            description: "A masked field with a show / hide toggle and an animated strength meter.",
            covers: &["Origin UI: Password input", "Origin UI: Password strength", "shadcn/ui: Password Input"],
            code: r##"ui.add(PasswordInput::new(&mut pw).label("Password").strength_meter(true));"##,
            show: |ui| {
                ui.add_space(70.0);
                with(ui, "pw", || ("Glowkit2026".to_string(), String::new()), |ui, (pw, pw2)| {
                    center(ui, 340.0, |ui| {
                        ui.spacing_mut().item_spacing.y = 18.0;
                        ui.add(PasswordInput::new(pw).label("Password").strength_meter(true).width(340.0));
                        ui.add(PasswordInput::new(pw2).label("Confirm password").placeholder("Repeat password").width(340.0));
                    });
                });
            },
        },
        Demo {
            id: "switch",
            name: "Switch",
            category: C,
            description: "A spring-loaded toggle whose thumb stretches while pressed; three sizes, labels and descriptions.",
            covers: &["shadcn/ui: Switch", "Animate UI: Switch", "Origin UI: Switch"],
            code: r##"ui.add(Switch::new(&mut notifications).label("Push notifications")
    .description("Receive alerts on your devices."));
ui.add(Switch::new(&mut on).size(ControlSize::Lg));"##,
            show: |ui| {
                ui.add_space(30.0);
                with(ui, "sw", || [true, false, true, true, false, true], |ui, s| {
                    center(ui, 560.0, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 30.0;
                            panel(ui, 340.0, |ui| {
                                ui.spacing_mut().item_spacing.y = 16.0;
                                title(ui, "Notifications", "Choose what you want to hear about.");
                                ui.add_space(2.0);
                                let [a, b, c, ..] = &mut *s;
                                ui.add(Switch::new(a).label("Push notifications").description("Receive alerts on your devices."));
                                ui.add(Switch::new(b).label("Marketing emails").description("News, product updates and offers."));
                                ui.add(Switch::new(c).label("Security alerts").description("Always on for admins.").disabled(true));
                            });
                            col(ui, 160.0, |ui| {
                                ui.spacing_mut().item_spacing.y = 18.0;
                                ui.add_space(10.0);
                                caption(ui, "Sizes");
                                let [_, _, _, d, e, f] = &mut *s;
                                ui.add(Switch::new(d).size(ControlSize::Sm).label("Small"));
                                ui.add(Switch::new(e).label("Medium"));
                                ui.add(Switch::new(f).size(ControlSize::Lg).label("Large"));
                            });
                        });
                    });
                });
            },
        },
        Demo {
            id: "checkbox",
            name: "Checkbox",
            category: C,
            description: "A check mark that draws itself, plus an indeterminate parent that tracks its children.",
            covers: &["shadcn/ui: Checkbox", "Animate UI: Checkbox", "Origin UI: Checkbox"],
            code: r##"let all = tasks.iter().all(|t| *t);
let some = tasks.iter().any(|t| *t);
let mut parent = all;
if ui.add(Checkbox::new(&mut parent).label("Select all").indeterminate(some && !all)).changed() {
    tasks.iter_mut().for_each(|t| *t = parent);
}
ui.add(Checkbox::new(&mut tasks[0]).label("Write documentation"));"##,
            show: |ui| {
                ui.add_space(40.0);
                with(ui, "cb", || (vec![true, false, true, false], true), |ui, (tasks, terms)| {
                    center(ui, 560.0, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 30.0;
                            panel(ui, 300.0, |ui| {
                                ui.spacing_mut().item_spacing.y = 14.0;
                                let all = tasks.iter().all(|t| *t);
                                let some = tasks.iter().any(|t| *t);
                                let mut parent = all;
                                if ui.add(Checkbox::new(&mut parent).label("Release checklist").indeterminate(some && !all)).changed() {
                                    tasks.iter_mut().for_each(|t| *t = parent);
                                }
                                ui.indent("tasks", |ui| {
                                    ui.spacing_mut().item_spacing.y = 12.0;
                                    for (i, name) in ["Write documentation", "Run the test suite", "Record the demo video", "Publish to crates.io"].iter().enumerate() {
                                        ui.add(Checkbox::new(&mut tasks[i]).label(*name));
                                    }
                                });
                            });
                            col(ui, 220.0, |ui| {
                                ui.spacing_mut().item_spacing.y = 16.0;
                                ui.add_space(8.0);
                                ui.add(Checkbox::new(terms).label("Accept terms and conditions").description("You agree to our Terms of Service and Privacy Policy."));
                                ui.add(Checkbox::new(&mut true).label("Disabled").disabled(true));
                                ui.add(Checkbox::new(&mut true).label("Custom color").color(Theme::of(ui).success));
                            });
                        });
                    });
                });
            },
        },
        Demo {
            id: "slider",
            name: "Slider",
            category: C,
            description: "Single and range sliders with a value bubble that pops up while hovering or dragging.",
            covers: &["shadcn/ui: Slider", "Origin UI: Slider", "Origin UI: Range slider", "Animate UI: Slider"],
            code: r##"ui.add(Slider::new(&mut volume, 0.0..=100.0).suffix("%"));
ui.add(RangeSlider::new(&mut price, 0.0..=500.0).step(10.0).prefix("$"));
ui.add(Slider::new(&mut stars, 1.0..=5.0).step(1.0).ticks(5));"##,
            show: |ui| {
                ui.add_space(50.0);
                with(ui, "slider", || (64.0f32, [120.0f32, 360.0], 3.0f32), |ui, (v, r, s)| {
                    center(ui, 380.0, |ui| {
                        let t = Theme::of(ui);
                        ui.spacing_mut().item_spacing.y = 10.0;
                        ui.horizontal(|ui| {
                            ui.add(FieldLabel::new("Volume"));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.label(RichText::new(format!("{v:.0}%")).color(t.muted_foreground)));
                        });
                        ui.add(Slider::new(v, 0.0..=100.0).suffix("%").width(380.0));
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            ui.add(FieldLabel::new("Price range"));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| ui.label(RichText::new(format!("${:.0} – ${:.0}", r[0], r[1])).color(t.muted_foreground)));
                        });
                        ui.add(RangeSlider::new(r, 0.0..=500.0).step(10.0).prefix("$").width(380.0));
                        ui.add_space(14.0);
                        ui.add(FieldLabel::new("Rating threshold"));
                        ui.add(Slider::new(s, 1.0..=5.0).step(1.0).ticks(5).suffix(" ★").width(380.0));
                    });
                });
            },
        },
        Demo {
            id: "select",
            name: "Select & Combobox",
            category: C,
            description: "Dropdown selects with icons, status dots and details; the combobox adds search and keyboard navigation.",
            covers: &["shadcn/ui: Select", "shadcn/ui: Combobox", "Origin UI: Select", "Kibo UI: Combobox"],
            code: r##"ui.add(Select::new(&mut status, [
    SelectOption::new("Backlog").dot(gray),
    SelectOption::new("In progress").dot(blue),
    SelectOption::new("Done").dot(green),
]).label("Status"));
ui.add(Combobox::new(&mut framework, ["Next.js", "SvelteKit", "Nuxt", "Remix", "Astro"])
    .label("Framework"));"##,
            show: |ui| {
                ui.add_space(30.0);
                with(ui, "sel", || (Some(1usize), None::<usize>, Some(0usize)), |ui, (status, fw, tz)| {
                    let t = Theme::of(ui);
                    center(ui, 720.0, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 20.0;
                            ui.add(
                                Select::new(status, [SelectOption::new("Backlog").dot(t.muted_foreground), SelectOption::new("Todo").dot(t.foreground), SelectOption::new("In progress").dot(t.info), SelectOption::new("In review").dot(t.warning), SelectOption::new("Done").dot(t.success)])
                                    .label("Status")
                                    .width(220.0)
                                    .id_salt("status"),
                            );
                            ui.add(
                                Combobox::new(fw, [SelectOption::new("Next.js").detail("React"), SelectOption::new("SvelteKit").detail("Svelte"), SelectOption::new("Nuxt").detail("Vue"), SelectOption::new("Remix").detail("React"), SelectOption::new("Astro").detail("Any"), SelectOption::new("Leptos").detail("Rust"), SelectOption::new("Dioxus").detail("Rust")])
                                    .placeholder("Select framework…")
                                    .label("Framework")
                                    .width(220.0)
                                    .id_salt("fw"),
                            );
                            ui.add(
                                Select::new(tz, [SelectOption::new("Pacific Time").icon(UiIcon::Globe).detail("UTC−8"), SelectOption::new("Eastern Time").icon(UiIcon::Globe).detail("UTC−5"), SelectOption::new("London").icon(UiIcon::Globe).detail("UTC+0"), SelectOption::new("Berlin").icon(UiIcon::Globe).detail("UTC+1"), SelectOption::new("Tokyo").icon(UiIcon::Globe).detail("UTC+9")])
                                    .label("Time zone")
                                    .icon(UiIcon::Clock)
                                    .width(220.0)
                                    .id_salt("tz"),
                            );
                        });
                    });
                });
            },
        },
        Demo {
            id: "otp-input",
            name: "OTP Input",
            category: C,
            description: "One-time-code boxes that fill as you type, auto-advance with a blinking caret and accept paste.",
            covers: &["shadcn/ui: Input OTP", "Origin UI: OTP input", "Animate UI: Input OTP", "Bencho: One-time code"],
            code: r##"ui.add(OtpInput::new(&mut code).length(6).separator_after(3));"##,
            show: |ui| {
                ui.add_space(50.0);
                with(ui, "otp", || ("4821".to_string(), "93".to_string(), "1234".to_string()), |ui, (a, b, c)| {
                    center(ui, 360.0, |ui| {
                        ui.spacing_mut().item_spacing.y = 10.0;
                        ui.add(FieldLabel::new("Verification code"));
                        ui.add(OtpInput::new(a).length(6).separator_after(3));
                        super::field::field_description(ui, "Enter the 6-digit code we sent to ada@lovelace.dev");
                        ui.add_space(12.0);
                        ui.add(OtpInput::new(b).length(4).joined(true).box_size(vec2(44.0, 44.0)));
                        ui.add_space(12.0);
                        ui.add(OtpInput::new(c).length(4).error(true).masked(true));
                        super::field::field_error(ui, "Incorrect PIN. 2 attempts left.");
                    });
                });
            },
        },
        Demo {
            id: "tag-input",
            name: "Tag Input",
            category: C,
            description: "A field of removable chips; Enter or comma adds a tag, Backspace removes the last one.",
            covers: &["Origin UI: Input with tags", "Kibo UI: Tags", "Dice UI: Tags Input"],
            code: r##"ui.add(TagInput::new(&mut tags).label("Topics").placeholder("Add a topic…").max_tags(8));"##,
            show: |ui| {
                ui.add_space(70.0);
                with(ui, "tags", || vec!["rust".to_string(), "egui".to_string(), "ui".to_string(), "animation".to_string()], |ui, tags| {
                    center(ui, 440.0, |ui| {
                        ui.add(TagInput::new(tags).label("Topics").placeholder("Add a topic…").max_tags(8).width(440.0));
                    });
                });
            },
        },
        Demo {
            id: "rating",
            name: "Rating",
            category: C,
            description: "Star ratings with hover preview, half stars, springy hover and read-only display.",
            covers: &["Origin UI: Rating", "Kibo UI: Rating", "Uiverse: Star rating"],
            code: r##"ui.add(Rating::new(&mut stars).half(true).show_value(true));
ui.add(Rating::new(&mut score).max(10).size(18.0).color(hex("#EC4899")));"##,
            show: |ui| {
                ui.add_space(90.0);
                with(ui, "rating", || (3.5f32, 4.0f32, 7.0f32), |ui, (a, b, c)| {
                    center(ui, 300.0, |ui| {
                        ui.spacing_mut().item_spacing.y = 18.0;
                        ui.add(Rating::new(a).half(true).size(30.0).show_value(true));
                        ui.add(Rating::new(b).size(22.0).color(Theme::of(ui).accent));
                        ui.add(Rating::new(c).max(10).size(18.0).color(glowkit_core::hex("#EC4899")).read_only(true));
                    });
                });
            },
        },
        Demo {
            id: "color-picker",
            name: "Color Picker",
            category: C,
            description: "Saturation area, hue and alpha sliders, hex field and swatches; inline or as a popover.",
            covers: &["Kibo UI: Color Picker", "shadcn/ui: Color Picker", "Origin UI: Color picker"],
            code: r##"ui.add(ColorPicker::new(&mut color));
ui.add(ColorPicker::new(&mut color).popover(true));"##,
            show: |ui| {
                ui.add_space(4.0);
                with(ui, "cp", || egui::Color32::from_rgb(139, 92, 246), |ui, c| {
                    center(ui, 540.0, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 40.0;
                            panel(ui, 288.0, |ui| {
                                ui.add(ColorPicker::new(c));
                            });
                            col(ui, 200.0, |ui| {
                                ui.spacing_mut().item_spacing.y = 10.0;
                                ui.add_space(8.0);
                                caption(ui, "Popover");
                                ui.add(ColorPicker::new(c).popover(true));
                                ui.add_space(10.0);
                                caption(ui, "Preview");
                                ui.add(Button::new("Brand button").fill(*c));
                            });
                        });
                    });
                });
            },
        },
        Demo {
            id: "calendar",
            name: "Calendar & Date Picker",
            category: C,
            description: "Month grids with today, single and range selection and a sliding month transition; plus popover date pickers.",
            covers: &["shadcn/ui: Calendar", "shadcn/ui: Date Picker", "Origin UI: Date range picker", "Kibo UI: Mini Calendar"],
            code: r##"ui.add(Calendar::new(&mut day).today(today));
ui.add(Calendar::range(&mut from, &mut to).months(2));
ui.add(DatePicker::new(&mut due));
ui.add(DatePicker::range(&mut from, &mut to));"##,
            show: |ui| {
                ui.add_space(4.0);
                with(ui, "cal", || (Some(CalendarDate::new(2026, 11, 18)), Some(CalendarDate::new(2026, 11, 9)), Some(CalendarDate::new(2026, 11, 14)), None::<CalendarDate>), |ui, (day, a, b, due)| {
                    center(ui, 840.0, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 20.0;
                            ui.add(Calendar::new(day).today(TODAY).id_salt("single"));
                            ui.add(Calendar::range(a, b).today(TODAY).id_salt("range"));
                            col(ui, 240.0, |ui| {
                                ui.spacing_mut().item_spacing.y = 8.0;
                                ui.add(FieldLabel::new("Due date"));
                                ui.add(DatePicker::new(due).today(TODAY));
                                ui.add_space(10.0);
                                ui.add(FieldLabel::new("Trip dates"));
                                let (mut x, mut y) = (*a, *b);
                                ui.add(DatePicker::range(&mut x, &mut y).today(TODAY).width(240.0));
                                *a = x;
                                *b = y;
                            });
                        });
                    });
                });
            },
        },
        Demo {
            id: "file-dropzone",
            name: "File Dropzone",
            category: C,
            description: "A dashed drop area whose border marches while files hover, with an animated list of received files.",
            covers: &["Origin UI: File upload", "Kibo UI: Dropzone", "Dice UI: File Upload", "Uiverse: File input", "Aceternity UI: File Upload", "Bencho: Upload dropzone"],
            code: r##"ui.add(FileDropzone::new(&mut files)
    .hint("PNG, JPG or PDF up to 10 MB")
    .on_drop(|f| println!("got {:?}", f.path())));"##,
            show: |ui| {
                ui.add_space(4.0);
                with(ui, "drop", || vec!["hero-illustration.png".to_string(), "q3-report.pdf".to_string()], |ui, files| {
                    center(ui, 760.0, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 20.0;
                            col(ui, 370.0, |ui| {
                                ui.add(FileDropzone::new(files).hint("PNG, JPG or PDF up to 10 MB").width(370.0));
                            });
                            col(ui, 370.0, |ui| {
                                ui.add(FileDropzone::new(&mut Vec::new()).title("Drop files here").hint("Drag-hover state").force_active(true).width(370.0));
                            });
                        });
                    });
                });
            },
        },
    ]
}
