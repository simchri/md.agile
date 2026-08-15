use super::*;
use clap::Parser;

#[test]
fn tasks_is_alias_for_task_subcommand() {
    let cli = Cli::try_parse_from(["agile", "tasks", "next"])
        .expect("`agile tasks next` should parse as the `task next` subcommand");
    assert!(matches!(
        cli.command,
        Some(Command::Task {
            action: TaskAction::Next {
                address: None,
                mine: false,
                r#as: None
            }
        })
    ));
}

#[test]
fn when_plot_ascii_flag_is_supported() {
    let result = Cli::try_parse_from(["agile", "when", "--plot", "--next", "1", "--ascii"]);
    assert!(
        result.is_ok(),
        "`agile when --plot --next 1 --ascii` should be accepted: {:?}",
        result.err()
    );
}

#[test]
fn when_data_ascii_is_rejected_by_clap_parsing() {
    let result = Cli::try_parse_from(["agile", "when", "--data", "--ascii"]);
    assert!(
        result.is_err(),
        "`agile when --data --ascii` should be rejected since --ascii requires --plot"
    );
}

#[test]
fn when_data_fit_is_rejected_by_clap_parsing() {
    let result = Cli::try_parse_from(["agile", "when", "--data", "--fit"]);
    assert!(
        result.is_err(),
        "`agile when --data --fit` should be rejected since --fit requires --plot"
    );
}

#[test]
fn when_plot_no_color_flag_is_supported() {
    let result = Cli::try_parse_from(["agile", "when", "--plot", "--next", "1", "--no-color"]);
    assert!(
        result.is_ok(),
        "`agile when --plot --next 1 --no-color` should be accepted: {:?}",
        result.err()
    );
}

#[test]
fn when_data_no_color_is_rejected_by_clap_parsing() {
    let result = Cli::try_parse_from(["agile", "when", "--data", "--no-color"]);
    assert!(
        result.is_err(),
        "`agile when --data --no-color` should be rejected since --no-color requires --plot"
    );
}
