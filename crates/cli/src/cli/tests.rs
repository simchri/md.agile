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
fn when_plot_ascii_flag_is_not_supported() {
    let result = Cli::try_parse_from(["agile", "when", "--plot", "--next", "1", "--ascii"]);
    assert!(
        result.is_err(),
        "`agile when --plot --next 1 --ascii` should be rejected"
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
