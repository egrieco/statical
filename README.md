# statical

A calendar aggregator and generator to make maintaining calendars on static websites easier.

## TODOs

- [ ] Add toml config
  - [ ] top level config object should be a site
  - [ ] paths for time intervals should be configurable
- [X] ~~*Add [tera](https://lib.rs/crates/tera) templates*~~ [2022-05-17]
- [ ] Add call to get first X day of the month
- [X] ~~*Add call to get date of the first day of the week*~~ [2022-05-19]
- [ ] Output html pages
  - [ ] event detail
  - [ ] agenda (list of events)
  - [ ] day
  - [X] ~~*week*~~ [2022-05-19]
  - [ ] month
  - [ ] quarter?
  - [ ] year?
  - [ ] index pages for each time interval
  - [X] ~~*link pages with forward and back links*~~ [2022-05-19]
  - [ ] add a sparse setting and decide how to handle missing intervals
  - [X] ~~*add default CSS*~~ [2022-05-19]
  - [ ] add links to switch between intervals
- [ ] Styling
  - [ ] Add weekday vs weekend classes
  - [ ] Figure out how to layout overlapping events. CSS grid to the rescue?
  - [ ] highlight current day
  - [ ] add event classes
  - [ ] add source calendar
  - [ ] add JS to toggle display of events by calendar
  - [ ] add JS to toggle display of events by category
- [ ] add an option to generate example templates or provide them in the docs/repo
