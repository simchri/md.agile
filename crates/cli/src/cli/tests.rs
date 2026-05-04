use super::*;
use clap::Parser;

#[test]
fn tasks_is_alias_for_task_subcommand() {
    let cli = Cli::try_parse_from(["agile", "tasks", "next"])
        .expect("`agile tasks next` should parse as the `task next` subcommand");
    assert!(matches!(
        cli.command,
        Some(Command::Task {
            action: TaskAction::Next
        })
    ));
}
