# statical

A calendar aggregator and generator to make maintaining calendars on static websites easier.

## Status

This program is now almost usable for basic calendar generation functionality. It is thus now in the late [Alpha](https://en.wikipedia.org/wiki/Software_release_life_cycle#Pre-alpha) stage of development. One contributor is apparently already using it regularly. The default templates are starting to look decent and are just about usable without edits.

The documentation needs to be completed as well as adding example config files and a setup guide. The code is now useful for users willing to tinker and dig, but most users should wait until the 1.0 version is released and the documentation is more complete.

A new version will be pushed to [crates.io](https://crates.io/crates/statical) shortly.

## Use

Use options `-f <file>` or `-u <url>` to specify the ICS file. The templates must be in `./templates/`. The config file `./statical.toml` will be created if needed.

## TODOs

- [ ] Calculate beginning and end dates of each calendar, do not default to today
- [ ] Add HTML sanitization to calendar descriptions
- [ ] Add support for Markdown in calendar descriptions
- [ ] Add ics feed generation
- [ ] Add keybindings to allow keyboard navigation of calendar
- [x] ~~_Switch week view to BTreeMap based event lists_~~ (2023-09-02)
- [x] ~~_Switch day view to BTreeMap based event lists_~~ (2023-09-02)
- [ ] Switch agenda view to BTreeMap based event lists (if needed)
- [ ] Loop through all months, weeks, days in the calendar ranges
- [ ] Add a sparse flag to not render missing intervals or to put placeholders there
- [x] ~~_Redo event grouping logic_~~ (2023-08-28)
  - [x] ~~_Store all events in a BTreeMap (it allows efficient in-order access and thus ranges)_~~ (2023-08-28)
  - [x] ~~_This should allow a single map to hold all events rather than the complex, nested structures we are using now_~~ (2023-08-28)
  - [x] ~~_Retrieve events from the map on view creation, maybe group them into relevant contexts then_~~ (2023-08-28)
- [x] ~~_Add styling to hide event descriptions in the calendar view and show them on hover_~~ (2023-09-01)
- [ ] Add JavaScript (or CSS toggle) to toggle event descriptions for mobile
- [ ] Add JavaScript to jump to the closest date to the one selected when switching view formats
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
- [x] Add toml config
  - [ ] top level config object should be a site
  - [ ] paths for time intervals should be configurable
  - [ ] calendar colors and CSS classes
- [x] ~~_Add [tera](https://lib.rs/crates/tera) templates_~~ (2022-05-17)
- [ ] Add call to get first X day of the month
- [x] ~~_Add call to get date of the first day of the week_~~ (2022-05-19)
- [ ] Output html pages
  - [ ] event detail
  - [x] agenda (list of events)
  - [x] ~~_day_~~ (2022-09-15)
  - [x] ~~_week_~~ (2022-05-19)
  - [x] ~~_month_~~ (2022-09-15)
  - [ ] quarter?
  - [ ] year?
  - [x] ~~_index pages for each time interval_~~ (2022-09-15)
  - [x] ~~_link pages with forward and back links_~~ (2022-05-19)
  - [ ] add a sparse setting and decide how to handle missing intervals
  - [ ] add a dense HTML calendar generation setting
  - [x] ~~_add default CSS_~~ (2022-05-19)
  - [ ] cleanup css
  - [x] ~~_add links to switch between intervals_~~ (2022-09-15)
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
- [x] ~~_add an option to generate example templates or provide them in the docs/repo_~~ (2023-09-04)

## Related Projects

- [zerocal](https://endler.dev/2022/zerocal/): A Serverless Calendar App in Rust Running on shuttle.rs by Matthias Endler
