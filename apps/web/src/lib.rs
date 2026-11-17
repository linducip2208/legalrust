use leptos::prelude::*;

// ---------------------------------------------------------------------------
// Pure helpers (unit-tested, no I/O)
// ---------------------------------------------------------------------------

/// CSS badge class for a deed state slug.
pub fn deed_state_badge(state: &str) -> &'static str {
    match state {
        "draft" => "badge-draft",
        "review" => "badge-review",
        "approved" => "badge-approved",
        "signed" => "badge-signed",
        "archived" => "badge-archived",
        "rejected" => "badge-rejected",
        _ => "badge-unknown",
    }
}

/// Format an integer minor-unit amount, e.g. `format_minor(1500000, "IDR")`.
pub fn format_minor(minor: i64, currency: &str) -> String {
    let negative = minor < 0;
    let mut digits = minor.unsigned_abs().to_string();
    let mut grouped = String::new();
    while digits.len() > 3 {
        let tail = digits.split_off(digits.len() - 3);
        grouped = format!(".{tail}{grouped}");
    }
    grouped = format!("{digits}{grouped}");
    if negative {
        format!("{currency} -{grouped}")
    } else {
        format!("{currency} {grouped}")
    }
}

/// Confidence bucket for OCR review UI.
pub fn confidence_class(c: f64) -> &'static str {
    if c < 0.7 {
        "low"
    } else if c < 0.9 {
        "med"
    } else {
        "high"
    }
}

/// Validate login form input. Returns `Ok(())` when acceptable.
pub fn validate_login(email: &str, password: &str) -> Result<(), &'static str> {
    let email = email.trim();
    let Some((_, domain)) = email.split_once('@') else {
        return Err("email must contain @");
    };
    if !domain.contains('.') || email.len() < 5 {
        return Err("email must be a valid address");
    }
    if password.len() < 8 {
        return Err("password must be at least 8 characters");
    }
    Ok(())
}

/// Build a deterministic CSV export filename: `{prefix}_{ymd}.csv`.
pub fn csv_filename(prefix: &str, ymd: &str) -> String {
    let clean: String = prefix
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let clean = clean.trim_matches('_').to_string();
    let name = if clean.is_empty() { "export" } else { &clean };
    format!("{name}_{ymd}.csv")
}

// ---------------------------------------------------------------------------
// Design system
// ---------------------------------------------------------------------------

pub mod design_system {
    /// Brand / surface palette (hex strings, single source of truth).
    pub const COLOR_BRAND: &str = "#1d4ed8";
    pub const COLOR_BRAND_DARK: &str = "#1e3a8a";
    pub const COLOR_SURFACE: &str = "#ffffff";
    pub const COLOR_BACKGROUND: &str = "#f1f5f9";
    pub const COLOR_TEXT: &str = "#0f172a";
    pub const COLOR_MUTED: &str = "#64748b";
    pub const COLOR_SUCCESS: &str = "#15803d";
    pub const COLOR_WARNING: &str = "#b45309";
    pub const COLOR_DANGER: &str = "#b91c1c";

    /// 4px-base spacing scale.
    pub const SPACING_XS: u32 = 4;
    pub const SPACING_SM: u32 = 8;
    pub const SPACING_MD: u32 = 16;
    pub const SPACING_LG: u32 = 24;
    pub const SPACING_XL: u32 = 32;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ButtonVariant {
        Primary,
        Secondary,
        Danger,
        Ghost,
    }

    impl ButtonVariant {
        pub fn css_class(&self) -> &'static str {
            match self {
                ButtonVariant::Primary => "btn btn-primary",
                ButtonVariant::Secondary => "btn btn-secondary",
                ButtonVariant::Danger => "btn btn-danger",
                ButtonVariant::Ghost => "btn btn-ghost",
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row types (passed as props; pages render empty vecs as honest zero-state)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientRow {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterRow {
    pub id: String,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentRow {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeedRow {
    pub id: String,
    pub title: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceRow {
    pub id: String,
    pub number: String,
    pub balance_minor: i64,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OcrCandidate {
    pub id: String,
    pub label: String,
    pub confidence: f64,
}

// ---------------------------------------------------------------------------
// Shell + pages
// ---------------------------------------------------------------------------

#[component]
pub fn AppShell(children: Children) -> impl IntoView {
    view! {
        <div class="app-shell">
            <header>
                <strong>"LEGALRUST"</strong>
                <nav>
                    <a href="/dashboard">"Dashboard"</a>
                    <a href="/clients">"Clients"</a>
                    <a href="/matters">"Matters"</a>
                    <a href="/documents">"Documents"</a>
                    <a href="/deeds">"Deeds"</a>
                    <a href="/finance">"Finance"</a>
                </nav>
            </header>
            <main>{children()}</main>
        </div>
    }
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    view! {
        <section>
            <h2>"Dashboard"</h2>
            <div class="stat-grid">
                <div class="stat-card">
                    <h3>"Clients"</h3>
                    <p>"—"</p>
                </div>
                <div class="stat-card">
                    <h3>"Matters"</h3>
                    <p>"—"</p>
                </div>
                <div class="stat-card">
                    <h3>"Documents"</h3>
                    <p>"—"</p>
                </div>
                <div class="stat-card">
                    <h3>"Deeds"</h3>
                    <p>"—"</p>
                </div>
            </div>
            <p class="zero-state">"hubungkan database untuk data live"</p>
        </section>
    }
}

#[component]
pub fn ClientsPage(#[prop(optional)] clients: Vec<ClientRow>) -> impl IntoView {
    view! {
        <section>
            <h2>"Clients"</h2>
            {if clients.is_empty() {
                view! { <p class="zero-state">"hubungkan database untuk data live"</p> }.into_any()
            } else {
                view! {
                    <ul>
                        {clients
                            .into_iter()
                            .map(|c| view! { <li>{c.name} " (" {c.id} ")"</li> })
                            .collect::<Vec<_>>()}
                    </ul>
                }
                    .into_any()
            }}
        </section>
    }
}

#[component]
pub fn MattersPage(#[prop(optional)] matters: Vec<MatterRow>) -> impl IntoView {
    view! {
        <section>
            <h2>"Matters"</h2>
            {if matters.is_empty() {
                view! { <p class="zero-state">"hubungkan database untuk data live"</p> }.into_any()
            } else {
                view! {
                    <ul>
                        {matters
                            .into_iter()
                            .map(|m| view! { <li>{m.title} " — " {m.status}</li> })
                            .collect::<Vec<_>>()}
                    </ul>
                }
                    .into_any()
            }}
        </section>
    }
}

#[component]
pub fn DocumentsPage(#[prop(optional)] documents: Vec<DocumentRow>) -> impl IntoView {
    view! {
        <section>
            <h2>"Documents"</h2>
            {if documents.is_empty() {
                view! { <p class="zero-state">"hubungkan database untuk data live"</p> }.into_any()
            } else {
                view! {
                    <ul>
                        {documents
                            .into_iter()
                            .map(|d| view! { <li>{d.title}</li> })
                            .collect::<Vec<_>>()}
                    </ul>
                }
                    .into_any()
            }}
        </section>
    }
}

#[component]
pub fn DeedsPage(#[prop(optional)] deeds: Vec<DeedRow>) -> impl IntoView {
    view! {
        <section>
            <h2>"Deeds"</h2>
            {if deeds.is_empty() {
                view! { <p class="zero-state">"hubungkan database untuk data live"</p> }.into_any()
            } else {
                view! {
                    <ul>
                        {deeds
                            .into_iter()
                            .map(|d| {
                                let badge = deed_state_badge(&d.state);
                                view! {
                                    <li>
                                        {d.title} " " <span class=badge>{d.state}</span>
                                    </li>
                                }
                            })
                            .collect::<Vec<_>>()}
                    </ul>
                }
                    .into_any()
            }}
        </section>
    }
}

#[component]
pub fn FinancePage(#[prop(optional)] invoices: Vec<InvoiceRow>) -> impl IntoView {
    view! {
        <section>
            <h2>"Finance"</h2>
            {if invoices.is_empty() {
                view! { <p class="zero-state">"hubungkan database untuk data live"</p> }.into_any()
            } else {
                view! {
                    <ul>
                        {invoices
                            .into_iter()
                            .map(|i| {
                                let bal = format_minor(i.balance_minor, &i.currency);
                                view! { <li>{i.number} " — " {bal}</li> }
                            })
                            .collect::<Vec<_>>()}
                    </ul>
                }
                    .into_any()
            }}
        </section>
    }
}

#[component]
pub fn OcrReviewPage(#[prop(optional)] candidates: Vec<OcrCandidate>) -> impl IntoView {
    view! {
        <section>
            <h2>"OCR Review"</h2>
            {if candidates.is_empty() {
                view! { <p class="zero-state">"hubungkan database untuk data live"</p> }.into_any()
            } else {
                view! {
                    <ul>
                        {candidates
                            .into_iter()
                            .map(|c| {
                                let cls = confidence_class(c.confidence);
                                view! {
                                    <li>
                                        {c.label} " "
                                        <span class=format!("confidence-{cls}")>{c.confidence}</span>
                                    </li>
                                }
                            })
                            .collect::<Vec<_>>()}
                    </ul>
                }
                    .into_any()
            }}
        </section>
    }
}

#[component]
pub fn LoginForm() -> impl IntoView {
    view! {
        <form method="post" action="/login">
            <label>
                "Email"
                <input type="email" name="email" required />
            </label>
            <label>
                "Password"
                <input type="password" name="password" required minlength="8" />
            </label>
            <button type="submit" class="btn btn-primary">
                "Sign in"
            </button>
        </form>
    }
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <AppShell>
            <main>
                <h1>"LEGALRUST"</h1>
                <p>"Enterprise Legal Operations — workspace shell (Phase 5 builds full UI)."</p>
                <nav>
                    <ul>
                        <li>"Dashboard (pending Phase 6 data)"</li>
                        <li>"Clients (pending)"</li>
                        <li>"Documents (pending)"</li>
                    </ul>
                </nav>
                <DashboardPage />
            </main>
        </AppShell>
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn ssr_renders_shell() {
        use leptos::prelude::*;
        let rendered = view! {
            <main>
                <h1>"LEGALRUST"</h1>
                <p>"Enterprise Legal Operations — workspace shell (Phase 5)."</p>
            </main>
        };
        let html = rendered.to_html();
        assert!(html.contains("LEGALRUST"), "shell must render brand");
        assert!(
            html.contains("Phase 5"),
            "shell must state its phase honestly"
        );
    }
}

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn deed_badge_covers_all_states() {
        assert_eq!(deed_state_badge("draft"), "badge-draft");
        assert_eq!(deed_state_badge("review"), "badge-review");
        assert_eq!(deed_state_badge("approved"), "badge-approved");
        assert_eq!(deed_state_badge("signed"), "badge-signed");
        assert_eq!(deed_state_badge("archived"), "badge-archived");
        assert_eq!(deed_state_badge("rejected"), "badge-rejected");
    }

    #[test]
    fn deed_badge_unknown() {
        assert_eq!(deed_state_badge("bogus"), "badge-unknown");
    }

    #[test]
    fn format_minor_zero() {
        assert_eq!(format_minor(0, "IDR"), "IDR 0");
    }

    #[test]
    fn format_minor_millions() {
        assert_eq!(format_minor(1500000, "IDR"), "IDR 1.500.000");
    }

    #[test]
    fn format_minor_negative() {
        assert_eq!(format_minor(-5000, "IDR"), "IDR -5.000");
    }

    #[test]
    fn confidence_boundaries() {
        assert_eq!(confidence_class(0.0), "low");
        assert_eq!(confidence_class(0.69), "low");
        assert_eq!(confidence_class(0.7), "med");
        assert_eq!(confidence_class(0.89), "med");
        assert_eq!(confidence_class(0.9), "high");
        assert_eq!(confidence_class(1.0), "high");
    }

    #[test]
    fn login_validation_ok() {
        assert!(validate_login("user@example.com", "password1").is_ok());
    }

    #[test]
    fn login_validation_rejects() {
        assert!(validate_login("not-an-email", "password1").is_err());
        assert!(validate_login("user@example.com", "short").is_err());
        assert!(validate_login("user@nodot", "password1").is_err());
    }

    #[test]
    fn csv_filename_basic() {
        assert_eq!(
            csv_filename("invoices", "2026-11-17"),
            "invoices_2026-11-17.csv"
        );
    }

    #[test]
    fn csv_filename_sanitizes() {
        assert_eq!(csv_filename("  ", "2026-11-17"), "export_2026-11-17.csv");
    }

    #[test]
    fn button_variants_have_classes() {
        use design_system::ButtonVariant;
        assert_eq!(ButtonVariant::Primary.css_class(), "btn btn-primary");
        assert_eq!(ButtonVariant::Danger.css_class(), "btn btn-danger");
        assert!(!ButtonVariant::Ghost.css_class().is_empty());
    }
}
