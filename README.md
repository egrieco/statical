# statical

A calendar aggregator and generator to make maintaining calendars on static websites easier.

## Status

This program is still in the [Pre-alpha](https://en.wikipedia.org/wiki/Software_release_life_cycle#Pre-alpha) stage of development. While it is almost working for me, it does not have a complete configuration system nor much in the way of documentation.

You are welcome to play with the code and attempt to use it, but most users should wait until the 1.0 version is released and the documentation is more complete.

## Use

Use options `-f <file>` or `-u <url>` to specify the ICS file. The templates must be in `./templates/`. The config file `./statical.toml` will be created if needed.

## TODOs

- [ ] Add ics feed generation
- [ ] Calendar filtering and processing
  - [ ] Event de-duplication
  - [ ] Event information merging
  - [ ] Information merge precedence/hierarchy
  - [ ] Add processing rules
    - [ ] Add categories
    - [ ] Add tags?
    - [ ] Hide/merge events
    - [ ] Move/copy/edit events
    - [ ] Add calendar groups
    - [ ] Calendar feed routing
- [X] Add toml config
  - [ ] top level config object should be a site
  - [ ] paths for time intervals should be configurable
  - [ ] calendar colors and CSS classes
- [X] ~~*Add [tera](https://lib.rs/crates/tera) templates*~~ [2022-05-17]
- [ ] Add call to get first X day of the month
- [X] ~~*Add call to get date of the first day of the week*~~ [2022-05-19]
- [ ] Output html pages
  - [ ] event detail
  - [X] agenda (list of events)
  - [X] ~~*day*~~ [2022-09-15]
  - [X] ~~*week*~~ [2022-05-19]
  - [X] ~~*month*~~ [2022-09-15]
  - [ ] quarter?
  - [ ] year?
  - [X] ~~*index pages for each time interval*~~ [2022-09-15]
  - [X] ~~*link pages with forward and back links*~~ [2022-05-19]
  - [ ] add a sparse setting and decide how to handle missing intervals
  - [ ] add a dense HTML calendar generation setting
  - [X] ~~*add default CSS*~~ [2022-05-19]
  - [ ] cleanup css
  - [X] ~~*add links to switch between intervals*~~ [2022-09-15]
- [ ] Styling
  - [ ] Add weekday vs weekend classes
  - [ ] Figure out how to layout overlapping events. CSS grid to the rescue?
  - [ ] highlight current day
  - [ ] add event classes
  - [ ] add source calendar
  - [ ] add event categories
- [ ] Add JavaScript
  - [ ] jump to current day
  - [ ] highlight current day
  - [ ] select day(s)
  - [ ] highlight selected day(s)
  - [ ] switch views while maintaining selected day(s)
  - [ ] add JS to toggle display of events by calendar
  - [ ] add JS to toggle display of events by category
- [ ] add an option to generate example templates or provide them in the docs/repo
