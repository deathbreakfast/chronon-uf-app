//! Mount Chronon ops pages for Playwright.

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;
use uf_integrations::{
    provide_shell_auth_menu, HostAuthMenu, ShellAppBar, ShellAuthMenu, UnifiedFieldAppBar,
    UnifiedFieldShellLayout,
};
use uf_product::components::ContentContainer;
use uf_product::primitives::{Body1, Flex, FlexAlign, FlexGap, Link, Title3};
use uf_product::{orbital_shell, OrbitalTemplate};

use crate::chronon_routes_eager::ChrononRoutesEager;
use crate::gate_demos::E2eAuthProvider;
use crate::harness_auth_menu::HarnessAuthMenu;

/// SSR document shell.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    orbital_shell(options, || view! { <App/> })
}

/// Root: harness auth + eager Chronon routes.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    chronon_app::ensure_help_steps_linked();
    uf_help::ensure_linked();
    #[cfg(feature = "ssr")]
    {
        provide_context(crate::e2e_higgs_config());
        provide_context(crate::e2e_chronon_backend());
        provide_context(crate::e2e_registry());
        provide_context(crate::e2e_pool_provider());
        wire_gauge_permissions_bridge();
    }
    provide_shell_auth_menu(|| view! { <HarnessAuthMenu /> });

    view! {
        <OrbitalTemplate>
            <Stylesheet id="leptos" href="/pkg/chronon-uf-app-e2e.css"/>
            <Title text="chronon-uf-app e2e"/>
            <E2eAuthProvider>
                <Router>
                    <Routes fallback=|| view! { <p>"Not found"</p> }>
                        <Route path=path!("/") view=HomePage/>
                        <ChrononRoutesEager />
                    </Routes>
                </Router>
            </E2eAuthProvider>
        </OrbitalTemplate>
    }
}

/// Wire Gauge as uf-product's permission backend (same pattern as boson-uf-app-e2e), as the
/// per-job access backend, and the Chronon admin group the job pages name.
#[cfg(feature = "ssr")]
pub fn wire_gauge_permissions_bridge() {
    use std::sync::Arc;

    struct GaugePermissionBackend;

    #[async_trait::async_trait]
    impl uf_product::permissions::PermissionBackend for GaugePermissionBackend {
        async fn has_permission(&self, permission_name: &str) -> Result<bool, ServerFnError> {
            let ctx = higgs::Higgs::from_request().await?;
            let valence = ctx
                .valence()
                .map_err(|e| ServerFnError::new(format!("Failed to build Valence: {e}")))?;
            let _caller =
                gauge::instrumentation::PermissionCheckCallerGuard::new("permission_backend");
            gauge::service::actor_can(&valence, permission_name)
                .await
                .map_err(|e| ServerFnError::new(format!("Failed to check permission: {e}")))
        }
    }

    /// Lab users have no authenticator, so role changes pass the step-up gate here. The
    /// step-up prompt itself is covered by uf-product's own tests.
    struct LabStepUp;

    #[async_trait::async_trait]
    impl uf_product::permissions::StepUpBackend for LabStepUp {
        async fn require_window(&self) -> Result<(), ServerFnError> {
            Ok(())
        }
    }

    uf_product::permissions::provide_permission_backend(Arc::new(GaugePermissionBackend));
    uf_product::permissions::provide_step_up_backend(Arc::new(LabStepUp));
    uf_product::access::provide_resource_access_backend(Arc::new(
        crate::access_backend::GaugeResourceAccess::new(crate::CHRONON_ADMINS_GROUP_ID),
    ));
    provide_context(chronon_app::server::ChrononAdminGroup {
        id: crate::CHRONON_ADMINS_GROUP_ID.to_string(),
        label: crate::CHRONON_ADMINS_LABEL.to_string(),
    });
}

#[cfg(not(feature = "ssr"))]
pub fn wire_gauge_permissions_bridge() {}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <UnifiedFieldShellLayout>
            <ShellAppBar slot>
                <UnifiedFieldAppBar app_name="Chronon e2e".to_string()>
                    <ShellAuthMenu slot:auth_menu>
                        <HostAuthMenu />
                    </ShellAuthMenu>
                </UnifiedFieldAppBar>
            </ShellAppBar>
            <ContentContainer max_width="900px" data_testid="chronon-e2e-home">
                <Flex vertical=true gap=FlexGap::Medium align=FlexAlign::Start>
                    <Title3>"chronon-uf-app e2e"</Title3>
                    <Body1>"ChrononRoutes host for Playwright."</Body1>
                    <Link href="/chronon">"Open /chronon"</Link>
                </Flex>
            </ContentContainer>
        </UnifiedFieldShellLayout>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
    uf_product::hide_boot_loader();
}
