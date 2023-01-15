# statical

A calendar aggregator and generator to make maintaining calendars on static websites easier.

## Status

This program is now almost usable for basic calendar generation functionality. It is thus now in the late [Alpha](https://en.wikipedia.org/wiki/Software_release_life_cycle#Pre-alpha) stage of development. One contributor is apparently already using it regularly. The default templates are starting to look decent and are just about usable without edits.

The documentation needs to be completed as well as adding example config files and a setup guide. The code is now useful for users willing to tinker and dig, but most users should wait until the 1.0 version is released and the documentation is more complete.

A new version will be pushed to [crates.io](https://crates.io/crates/statical) shortly.

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

## Related Projects

- [zerocal](https://endler.dev/2022/zerocal/): A Serverless Calendar App in Rust Running on shuttle.rs by Matthias Endler
