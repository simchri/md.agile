# Manifesto

This file aims to explain major design decisions and the philosophy of this project. Mdagile is certainly an opinionated piece of software; you may not agree with all points in here. Take this file also as a help to figure out whether Mdagile is for you or not.

## Not a Web App
Web apps suck ***. Seriously, this project is inspired 80% by my personal hatred for web apps (Jira in particular). I don't know how we have come to largely accept laggy, unresponsive interfaces, reliance on a constant web connection, and total submission to the tool vendor's whims and wishes. No, I don't want a new UI design for my to-do list. I don't want to relearn how to use the interface. I want to get my work done. Yes, I would like to be able to see my tasks when I am offline. Thanks!

## Plain Text & Version Control
Managing tasks should be as simple as editing a few lines in a text file, then committing and pushing the change. Plain text is fantastic: Everyone, even your PO, knows how to edit a text file. If not, you can teach them in 5 minutes (version control and syncing is a different topic, of course).

Nobody needs to be taught how to work with a file that looks like this:

```md
- [ ] confirmation dialog before shutdown
  - [x] implementation
  - [ ] testing
```
The moment you have seen this example, you know how to work with it.

## Living in the Real World
We want to build an agile task management tool from developers for developers. We want to live in a better world. This, however, also requires seeing and understanding the current state of the real world. We have to offer an on-ramp for all the other people on the project who are not developers. It must be easy to interact with the task list, even without understanding the intricacies of a version control system. Otherwise, this will never be adopted and we will be stuck with Jira.

It must be possible to easily attach a GUI. This GUI should handle git pull and push automatically, and it must run on Windows also.

## Actually Synchronized
Ever seen a ticket that was "done" and wondered in which version the feature would actually be available? Have you had the pleasure yet of trying to configure your CI to integrate with Jira?
Manage your tasks in your repo. Mark them as "done" in the same commit where the feature is implemented! You will always know exactly where the project stands.

## No Story Points - #NoEstimates
This project does not and will not have support for story points or sprints. Scrum is an antipattern.
There is a better way to do Agile, and we can promote healthier, more effective software work by providing suitable tools (Jira is not suitable). Therefore, this project comes with estimation features in line with Allen Holub's #NoEstimates concept: Provide Estimated Time of Arrival based simply on task counts and past performance (but don't estimate individual tasks).
This approach does not rely on guesswork or forced promises. It promotes healthier relationships between people and recognizes the uncertainty inherent to knowledge work.

In most projects, some form of estimation is going to take place. Someone, somewhere, is going to want to know when the thing is done.
We are trying to provide *something* that is easy and convenient, but does not rely on guesswork, nor time boxing. The idea is simple: Pull management away from the most toxic methods, towards something at least slightly better. Yes, this is admitting defeat in some sense.
ETA estimation is only as good as the information available in the backlog. If no tasks are planned, if we don't know what to do, there is no ETA. You must at least try to outline what is still to be done; otherwise, there is no estimate. This tool offers a simple contract: I can estimate when I am done, when you can estimate what is left to do!

## Transparent, Flexible
A local todo list is easy to use, flexible, and informal. A centralized task management tool creates transparency, a common vision, and can be used to strengthen common rules for collaboration ("A task is done when ...").
Mdagile is a compromise between these approaches. You can quickly and easily take notes—but they will be shared with everyone. You can keep a task completely informal (just a bullet point)—or make it adhere to a predefined workflow. The decision can be made task by task.

## Trust but Control
Centralized task management allows you to restrict lifecycle transitions to specific users. This is difficult when everything is text-based. Mdagile provides some constraints, but does not aim to be "secure".
If necessary, you can analyze your git history to figure out if something went wrong.

## Processes are Version Controlled
Tasks' life cycles are defined in a version-controlled file. The team decides together how work is structured and changes this via pull requests!
The team agrees on a workflow; programmers translate it into specific, enforceable rules. Have you ever seen a well-configured Jira project? The lifecycle states made sense, the mandatory fields for each transition were just right? You could provide feedback and the project managers actually considered it and implemented it?
Yeah, me neither.

## Configurable - From Simple to Enterprise
Mdagile should encourage very simple, minimal task workflows, but also allow larger organizations to actually use this as a major collaboration tool. Users should be able to define "enterprise"-ready workflows via configuration.

## Be Strict, No Bike Shedding
We add some arbitrary strict rules to the task syntax, e.g., marking tasks can only be done with `x`, not with `X`. Allow users to sidestep any discussions about irrelevant details.

## What does this project do that MDTASK does not?
- Actually free license
- Controls and checks: Constrain the possible actions (sequence of tasks, tasks that can only be checked by authorized groups)
- ... generally just more features: This scales to complex workflows/lifecycles for tasks if needed. Hopefully your workflows will be simple, but they can also be "enterprise".
- But wait, I don't want enterprise workflows! — Me neither, but look—if your organization forces you into a complex process, it's going to force you into a complex process. You have the choice if you handle it in text files, or some web app.
