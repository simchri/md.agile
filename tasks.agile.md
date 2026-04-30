- [x] post initial vision!
- [x] refine and clean up

# More Cli stuff
- [x] for any "list" command, add arguments "--next <count>" / "-n <count>"  show only the first <count> entries of whatever is listed.
- [x] idem "--last" ...

- [x] add subcommand (alias) "agile list tasks" should do the same as "agile list"

- [x] data structures suitable for parsing
- [x] implement a proper parser for tasks
- [x] integrate the parser into existing functions
- [x] next task uses parser, "location" added to each task struct

## small cli fixes
- [x] "tasks" is an alias for task subcommand

## First basic checks
- [ ] agile check subcommand created
- [ ] Check for wrongly indented task (task that is surrounded by newlines but indented like a subtaks)
any operation that parses tasks lists should immediately stop on encountering this error. The error should be printed, including file path and line number

- [ ] iterate on the first error message
  - [ ] proper coloring
  - [ ] indenting
  - [ ] other highlighting
  Bottom Line: It should be a really ergonomic, nicely readable error message, but which could also be parsed (machine readable)
  
  - [ ] foo bad task


## More CLI features
- [ ] create a global overview of the CLI structure
  - [ ] list of subcommands and their functions
  - [ ] most important flags to each subcommand




