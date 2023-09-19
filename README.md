# statical

A calendar aggregator and generator to make maintaining calendars on static websites easier.

## Why this exists

While there is no shortage of calendaring services available, they all have drawbacks:

- They are branded, or rather expensive with prices ranging from around $8-45 per month. While that's not terrible on the low end, if you want to add calendaring to multiple websites it adds up quickly.
- These calendars, whether embedded or linked, call out to their servers which may not respect the privacy preferences of site visitors.
- While self-hosted options are available, I don't want to have to setup and administer another server just to host a simple calendar. Static sites are excellent and the calendar should be static as well.

None of the available options met my needs so I decided to build my own. This project was also excellent motivation and practice for Rust software development.

## Status

This program is now mostly usable for basic calendar generation functionality. It is thus now in the [Beta](https://en.wikipedia.org/wiki/Software_release_life_cycle#Beta) stage of development and nearing the version 1.0 milestone.

Statical can now generate an example config file with the `--generate-default-config` flag.
The documentation needs to be completed as well as adding a setup guide. The code is nearing general usability, but may still require a bit of tinkering and digging as a few key features are missing.

The default templates are starting to look acceptable and we are planning a final design pass shortly. While the templates and CSS are fully customizable, the defaults should produce a calendar with good user experience, interface and aesthetics.

## Features

- Reads `*.ics` files or live calendar feeds
- Does NOT require contributors to create a new login. Just add their calendar feed to the config file.
- Can be run manually on your personal machine or setup on a Cron job, Git hook, or Continuous Integration (CI) pipeline
- Generates static HTML views
  - Month
  - Week
  - Day
  - Agenda
- View customization
  - Default views are embedded in the app
  - Alternately, views can also be individually overridden and fully customized via Tera templates
  - Views are completely HTML and CSS based, no JavaScript is present
  - SASS/CSS is provided but can be edited or completely overridden
  - Calendars can be assigned custom colors. Any valid CSS color notation should work, including color names.
  - Colors are adjusted for readability via the [Oklch color space](https://lea.verou.me/blog/2020/04/lch-colors-in-css-what-why-and-how/#what-is-lch%3F). (The lightness and chroma adjustment values can be configured or adjustment can be entirely disabled.)
- Generates calendar feeds in ICS format

## Target users

Statical is built with three types of users in mind:

### 1. Casual CLI users

Statical should be easy enough for someone with basic CLI knowledge to install it, modify the default configuration, and have calendars that look good in minutes.

### 2. Advanced web designers

For those who want more control of the calendars generated:

- Ample config options are provided
- SASS/CSS can be customized or completely overridden
- Views are generated via templates that can be customized or completely overridden

### 3. Rust programmers

If complete control is desired, this code is released under the [BSD 3 clause license](LICENSE.txt).

## Usage

### Background

Statical is intended to be used in a "Static Site Generator chain" ([credit to CloudCannon for the term](https://cloudcannon.com/blog/introducing-pagefind/)). Statical should run before tools like Pagefind and Jampack as its output pages will need to be indexed and optimized.

An example chain might look like the following:

1. [soupault](https://soupault.app/) (or your favorite [static](https://www.smashingmagazine.com/2015/11/modern-static-website-generators-next-big-thing/) [site](https://jamstack.org/generators/) [generator](https://staticsitegenerators.net/))
2. **statical**
3. [Pagefind](https://pagefind.app/) (or [tinysearch](https://github.com/tinysearch/tinysearch), [stork](https://github.com/jameslittle230/stork), [orama](https://github.com/oramasearch/orama), or similar)
4. [Jampack](https://jampack.divriots.com/)
5. deploy or sync your site

### Configuration

Statical must have a config file is order to run. **Create the example config file** with the command:

```zsh
statical --create-default-config
```

**Edit the config file** as necessary with your favorite text editor. Only the keys below are strictly necessary:

- `display_timezone`: One of the TZ identifiers from the [IANA Time Zone Database](https://en.wikipedia.org/wiki/List_of_tz_database_time_zones#List)
- `default_calendar_view`: one of Month, Week, Day, or Agenda
- `calendar_sources` Multiple sources can be provided.
  - `name`: must be kebab-case
  - `source`: can be the URL of a calendar feed or a local `*.ics` file


The rest have default values that should work for most users. There are comments in the generated config file explaining the purpose of each option.

### Running statical

Statical needs to be run every time there are changes to the calendar. This can be done manually or via cron job, Git hook or CI pipeline.

Statical will look in the current directory for its config file named `statical.toml`. Alternately, you can specify one or more config files as arguments to statical e.g.:

```zsh
statical site-one/statical.toml site-two/statical.toml ...
```

### Customization

Default assets and templates are built into statical, but can be overridden by the user if desired.

#### Assets

If you would like to customize the CSS run:

```zsh
statical --restore-missing-assets
```

The assets directory will be created at the location specified in the configuration file (paths are relative to the config file itself). The default is `assets`.

Any missing asset files will be re-created from those built-in to statical. Assets can be edited as desired, or deleted to return to the built-in defaults. If an asset is already present with the same name as one of the defaults, it will not be overwritten.

#### Templates

To customize the views themselves run:

```zsh
statical --restore-missing-templates
```

The templates directory will be created at the location specified in the configuration file (paths are relative to the config file itself). The default is `templates`.

Any missing template files will be re-created from those built-in to statical. Templates can be edited as desired, or deleted to return to the built-in defaults. If a template is already present with the same name as one of the defaults, it will not be overwritten.

#### Template Language

Statical uses [Tera](https://keats.github.io/tera/) templates to allow customization of calendar views. For detailed information about Tera its capabilities see the [Tera Documentation](https://keats.github.io/tera/docs/).

To see what data is available for use within a given template, add the following code somewhere in your template:

```html
<pre>
  {{ __tera_context }}
</pre>
```

## Related Projects

If statical does not do exactly what you need, check out these projects instead.

- [ical-merger](https://lib.rs/crates/ical-merger): Merges multiple iCalendar files into one, as a web service.
- [ical-filter](https://lib.rs/crates/ical-filter): HTTP daemon to normalize and filter iCalendar files
- [zerocal](https://endler.dev/2022/zerocal/): A Serverless Calendar App in Rust Running on shuttle.rs by Matthias Endler

## Road map and TODOs

### Pre-release testing fixes

- [x] ~~_Fix default date bug_~~ (2023-09-13)
- [x] ~~_Make copy stylesheet true by default_~~ (2023-09-13)
- [x] ~~_Embed default stylesheet in binary_~~ (2023-09-13)
- [x] ~~_Add --restore-missing-templates_~~ (2023-09-14)
- [x] ~~_Make the config generation write to file directly_~~ (2023-09-14)
- [ ] Put sources default in that triggers help if it is not updated
- [ ] Add initial setup option
- [x] ~~_Add help when command is first run_~~ (2023-09-14)
- [ ] Add assistant to help add calendar sources?
- [ ] Ensure that partial configuration files work i.e. those missing many keys
- [x] ~~_Create a list of required config keys, the minimum necessary to run statical_~~ (2023-09-15)

### Setup and Configuration (1.0 Milestone)

- [x] Add toml config
- [x] ~~_add an option to generate example templates or provide them in the docs/repo_~~ (2023-09-04)
- [x] ~~_Add [tera](https://lib.rs/crates/tera) templates_~~ (2022-05-17)
- [x] ~~_add baseurl support_~~ (2023-09-08)
- [x] ~~_Default to looking for the `statical.toml` file in the current dir_~~ (2023-09-08)
- [x] ~~_Make all paths relative to the config file_~~ (2023-09-09)
- [x] ~~_Prompt with instructions on how to use Statical if config file is not present or provided._~~ (2023-09-14)
- [x] ~~_Add `--restore-missing-assets` option_~~ (2023-09-14)

### Setup and Configuration (Future Work)

- [x] ~~_Allow template path config._~~ (2023-09-14)
- [x] ~~_calendar colors and CSS classes_~~ (2023-09-19)
- [ ] paths for time interval pages should be configurable?

### Calendar Generation (1.0 Milestone)

- [x] ~~_Add call to get date of the first day of the week_~~ (2022-05-19)
- [x] ~~_Switch week view to BTreeMap based event lists_~~ (2023-09-02)
- [x] ~~_Switch day view to BTreeMap based event lists_~~ (2023-09-02)
- [x] ~~_Redo event grouping logic_~~ (2023-08-28)
  - [x] ~~_Store all events in a BTreeMap (it allows efficient in-order access and thus ranges)_~~ (2023-08-28)
  - [x] ~~_This should allow a single map to hold all events rather than the complex, nested structures we are using now_~~ (2023-08-28)
  - [x] ~~_Retrieve events from the map on view creation, maybe group them into relevant contexts then_~~ (2023-08-28)

### Styling (1.0 Milestone)

- [x] ~~_Add styling to hide event descriptions in the calendar view and show them on hover_~~ (2023-09-01)
- [x] ~~_Add weekday vs weekend classes_~~ (2023-09-08)
- [x] ~~_SASS processing_~~ (2023-09-19)
- [x] ~~_add source calendar class_~~ (2023-09-19)
- [ ] Remove no-wrap from event header text (but keep no-wrap on duration)
- [ ] highlight current day
- [ ] Clean up pagination and views
- [ ] Align pagination with grid
- [ ] Center header
- [ ] cleanup css
- [ ] CSS classes for calendar colors

### Styling (Future Work)

- [ ] add event classes
- [ ] add event categories
- [ ] Figure out how to layout overlapping events. CSS grid to the rescue?
- [ ] Make overlapping events stack horizontally in the Day view on desktop (maybe week and month if space allows)
- [ ] Add times on left side and align events in week and day view

### Output pages (1.0 Milestone)

- [x] agenda (list of events)
- [x] ~~_day_~~ (2022-09-15)
- [x] ~~_week_~~ (2022-05-19)
- [x] ~~_month_~~ (2022-09-15)
- [x] ~~_index pages for each time interval_~~ (2022-09-15)
- [x] ~~_link pages with forward and back links_~~ (2022-05-19)
- [x] ~~_add default CSS_~~ (2022-05-19)
- [x] ~~_add links to switch between intervals_~~ (2022-09-15)
- [x] ~~_Add summary to event header_~~ (2023-09-08)
- [x] ~~_Store templates internally but use external versions if provided._~~ (2023-09-08)
- [ ] event detail
  - [ ] decide on url naming, probably not date based, maybe including calendar name
  - [ ] use unexpanded events
- [ ] Add ics feed generation
- [x] ~~_Add month name on fist day of month in week view (just like month view)_~~ (2023-09-09)
- [x] ~~_Determine which month a week "belongs to" based on which month has the most days in that week?_~~ (2023-09-09)
- [ ] Clean up the HTML class logic in the week template, move it into the Week class when generating contexts
- [ ] Add day strftime format?
- [ ] Add strftime format for agenda dates?
- [ ] Add keybindings to allow keyboard navigation of calendar

### JavaScript (Future Work)

- [ ] Add JavaScript (or CSS toggle) to toggle event descriptions for mobile
- [ ] Add JavaScript to jump to the closest date to the one selected when switching view formats
- [ ] jump to current day
- [ ] highlight current day
- [ ] select day(s)
- [ ] highlight selected day(s)
- [ ] switch views while maintaining selected day(s)
- [ ] add JS to toggle display of events by calendar
- [ ] add JS to toggle display of events by category

### Calendar generation (1.0 Milestone)

- [ ] Fix agenda event collection logic
- [ ] Fix event ordering in day view
- [ ] Calculate beginning and end dates of each calendar, do not default to today

### Calendar Generation (Future Work)

- [ ] Loop through all months, weeks, days in the calendar ranges (dense HTML calendar generation setting)
- [ ] add a sparse setting and decide how to handle missing intervals
- [ ] Add a sparse flag to not render missing intervals or to put placeholders there

### Calendar filtering and processing (Future Work)

- [ ] Add HTML sanitization to calendar descriptions
- [ ] Add support for Markdown in calendar descriptions
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
- [ ] add human date format parsing
- [ ] support for sunrise and sunset (if event has a location, or default to calendar location?)
- [ ] Add support for first/second/third/etc. X day of the month

### Ergonomics

- [ ] live preview server

### External tool integration (Future Work)

- [ ] pagefind integration (add indexing hints templates)
- [ ] jampack integration?
