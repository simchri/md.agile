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
            div { class: "separator1" }
            div { class: "separator2" }

            div { class: "task-card", style: "top: 30px; left: 30px;",
                "Implement user\nauthentication"
            }
        }
    }
}
