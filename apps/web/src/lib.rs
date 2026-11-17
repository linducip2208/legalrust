use leptos::prelude::*;
#[component]
pub fn App() -> impl IntoView {
    view! {
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
        </main>
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
