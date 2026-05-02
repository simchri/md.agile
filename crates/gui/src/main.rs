use dioxus::prelude::*;

fn main() {
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");
    launch(App);
}

#[allow(non_snake_case)]
fn App() -> Element {
    rsx! {
        div { class: "layout",
            div { class: "top-row" }
            div { class: "separator" }
            div { class: "middle-row" }
            div { class: "separator" }
            div { class: "bottom-row" }
        }
    }
}
