use leptos::prelude::*;
use leptos_router::components::A;
use orbital::components::{Card, CardContent, ContentContainer, SpacingSize, Title3};
use orbital::primitives::{Button, ButtonAppearance, Flex, FlexAlign, FlexJustify};

use crate::components::{chronon_card_content, chronon_table_page_layout, JobsDataTable};
use crate::server::get_job_access;

/// Jobs list page
#[component]
pub fn ChrononJobsPage() -> impl IntoView {
    let (card_content_style, card_content_class) = chronon_card_content();
    let (page_style, page_classes) = chronon_table_page_layout();
    let fill_card_content = format!("{} {}", card_content_class, page_classes.card_content);
    let can_create = RwSignal::new(false);
    Effect::new(move |_| {
        leptos::task::spawn_local_scoped(async move {
            if let Ok(access) = get_job_access(None).await {
                can_create.set(access.can_create);
            }
        });
    });

    view! {
        <style>{card_content_style}</style>
        <style>{page_style}</style>
        <div id="chronon-jobs-page">
        <ContentContainer class=page_classes.page data_testid="chronon-jobs-page">
            <Flex vertical=true gap=SpacingSize::Size240.flex_gap() class=page_classes.body>
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                    <Title3>"Jobs"</Title3>
                    <Show when=move || can_create.get()>
                        <div id="chronon-jobs-create-button" data-testid="jobs-create-button">
                            <A href=crate::paths::JOBS_NEW>
                                <Button appearance=ButtonAppearance::Primary icon=icondata::AiPlusOutlined>
                                    "Create job"
                                </Button>
                            </A>
                        </div>
                    </Show>
                </Flex>

                <Card class=page_classes.card>
                    <CardContent class=fill_card_content>
                        <div id="chronon-jobs-search">
                            <JobsDataTable can_create=can_create />
                        </div>
                    </CardContent>
                </Card>
            </Flex>
        </ContentContainer>
        </div>
    }
}
