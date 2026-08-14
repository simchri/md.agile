# Prioritization with Properties

With Md Agile, there is no way around an absolute priority order (the tool always requires you to maintain this order).

BUT in real life, we sometimes don't care about a global order, or don't have the time to care or just have to ignore it, for reasons. Sometimes assigning broader priority "categories" is more convenient:

If you go through a long list of bugs trying to classify which ones are important and which ones aren't, you want to quickly sort them into two or three bins (e.g. high, medium, low). When a new bug comes in, you don't want to compare it against ALL other bugs to give it a fitting priority rank. You want to just say "high / medium / low".

You can very easily implement this, by defining properties `#high`,`#medium`,`#low` and assigning them to your bug-tasks. In you editor, just text-search for the property `#high`: The first result is your highest prio bug. Property filters are also planned for the board view.

It is fully up to you, if you then want to ALSO maintain an absolute priority order consistent with the property prios or not. (i.e. do you want to re-sort the bugs, so that in the text files all `#high` bugs appear before all `#medium` etc.). 

You may want to consider the impact on Milestones and ETA features! Recommendation: For each bug, at least decide explicitly, if you want it fixed for the next milestone or not. Otherwise the bug-fixing work is not correctly considered in your ETAs. 
