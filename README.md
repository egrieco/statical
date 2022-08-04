# statical

A calendar aggregator and generator to make maintaining calendars on static websites easier.

## Status

This program is still in the [Pre-alpha](https://en.wikipedia.org/wiki/Software_release_life_cycle#Pre-alpha) stage of development. While it is almost working for me, it does not have a complete configuration system nor much in the way of documentation.

You are welcome to play with the code and attempt to use it, but most users should wait until the 1.0 version is released and the documentation is more complete.

## TODOs

- [ ] Add toml config
  - [ ] top level config object should be a site
  - [ ] paths for time intervals should be configurable
  - [ ] agenda page size
  - [ ] agenda start date
- [X] ~~*Add [tera](https://lib.rs/crates/tera) templates*~~ [2022-05-17]
- [ ] Add call to get first X day of the month
- [X] ~~*Add call to get date of the first day of the week*~~ [2022-05-19]
- [ ] Output html pages
  - [ ] event detail
  - [X] agenda (list of events)
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
