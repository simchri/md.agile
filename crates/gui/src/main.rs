use dioxus::prelude::*;
use log::info;

mod server;

fn main() {
    init_logger();
    info!("mdagile-gui main");

    dioxus::launch(app);
}

fn init_logger() {
    #[cfg(not(feature = "web"))]
    env_logger::init();

    #[cfg(feature = "web")]
    console_log::init_with_level(log::Level::Debug).expect("error initializing logger");
}

fn app() -> Element {
    
    let mut next = use_resource(|| async { server::get_next_task().await });

    let title = match &*next.read_unchecked() {
        Some(Ok(Some(t))) => t.clone(),
        Some(Ok(None))    => "All tasks done".to_string(),
        Some(Err(e))      => format!("Error: {e}"),
        None              => "Loading…".to_string(),
    };

    use_effect({
        // Clock, frequency 1s.
        // Poll updates from the server side (e.g. update task list)
        move || {
            dioxus::prelude::spawn(async move {
                log::info!("use_effect: clock START");
                use wasmtimer::tokio::sleep;

                loop {
                    sleep(std::time::Duration::from_millis(1000)).await;

                    // updates the "next" resource, by re-evaluating the registered function, therefore calling the backend, to get the current latest task.
                    next.restart(); 
                }
            });
        }
    });

    rsx! {
        div { class: "layout",
            div { class: "separator1" }
            div { class: "separator2" }

            div { class: "task-card", style: "top: 30px; left: 30px;",
                "{title}"
            }
        }
    }
}
