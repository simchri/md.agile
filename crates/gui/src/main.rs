use dioxus::prelude::*;
use log::info;

fn main() {
    #[cfg(not(feature = "web"))]
    env_logger::init();

    #[cfg(feature = "web")]
    console_log::init_with_level(log::Level::Debug).expect("error initializing logger");

    info!("main()");

    info!("Launch Dioxus App");
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div { class: "layout",
            div { class: "top-row",
                "TEST: Backlog section"
                div { class: "task-card",
                    "Implement user authentication"
                }
            }
            div { class: "separator" }
            div { class: "middle-row",
                "TEST: In Progress section"
            }
            div { class: "separator" }
            div { class: "bottom-row",
                "TEST: Done section"
            }
        }
    }
}
