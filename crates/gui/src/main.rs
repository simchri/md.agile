use dioxus::prelude::*;

fn main() {
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");
    launch(App);
}

#[allow(non_snake_case)]
fn App() -> Element {
    rsx! {
        head {
            meta { charset: "utf-8" }
            meta { name: "viewport", content: "width=device-width, initial-scale=1" }
            title { "mdagile GUI" }
            style { {CSS} }
        }
        body {
            div { class: "layout",
                div { class: "top-row" }
                div { class: "separator" }
                div { class: "middle-row" }
                div { class: "separator" }
                div { class: "bottom-row" }
            }
        }
    }
}

const CSS: &str = r#"
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background: white;
    height: 100vh;
    display: flex;
}

.layout {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
}

.top-row {
    flex: 0 0 15%;
    border-bottom: 1px solid black;
}

.separator {
    flex: 0 0 1px;
    background: black;
}

.middle-row {
    flex: 1;
    border-bottom: 1px solid black;
}

.bottom-row {
    flex: 0 0 15%;
}
"#;
