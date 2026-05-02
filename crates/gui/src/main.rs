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

fn get_next_task(content: &str) -> Option<String> {
    for line in content.lines() {
        if line.starts_with("- [ ]") {
            return Some(line.strip_prefix("- [ ] ")?.trim().to_string());
        }
    }
    None
}

#[component]
fn App() -> Element {
    let mut task = use_signal(|| "Loading...".to_string());

    use_effect(move || {
        let task = task;
        spawn({
            async move {
                match gloo_net::http::Request::get("/tasks.agile.md").send().await {
                    Ok(response) => {
                        match response.text().await {
                            Ok(content) => {
                                if let Some(t) = get_next_task(&content) {
                                    task.set(t);
                                } else {
                                    task.set("No pending tasks".to_string());
                                }
                            }
                            Err(_) => {
                                task.set("Error reading tasks".to_string());
                            }
                        }
                    }
                    Err(_) => {
                        task.set("Error fetching tasks".to_string());
                    }
                }
            }
        });
    });

    rsx! {
        div { class: "layout",
            div { class: "separator1" }
            div { class: "separator2" }

            div { class: "task-card", style: "top: 30px; left: 30px;",
                "{task()}"
            }
        }
    }
}
