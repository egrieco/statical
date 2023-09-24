use chrono::{DateTime, Datelike, Days, Duration, NaiveDate, Utc};
use chrono_tz::Tz as ChronoTz;
use chronoutil::DateRule;
use color_eyre::eyre::{self, bail, eyre, Context as EyreContext, Result};
use fuzzydate::parse;
use humantime::parse_duration;
use include_dir::{
    include_dir, Dir,
    DirEntry::{Dir as DirEnt, File as FileEnt},
};
use itertools::Itertools;
use log::{debug, error, info};
use lol_html::{element, html_content::ContentType, rewrite_str, Settings};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};
use std::{fs, iter};
use std::{
    fs::{create_dir_all, File},
    io::Write,
};
use std::{io::Read, rc::Rc};
use tera::{Context, Tera};

use super::calendar_source::CalendarSource;
use super::day::Day;
use super::event::{Event, EventList, UnparsedProperties};
use super::week::Week;
use crate::util::delete_dir_contents;
use crate::views::agenda_view::AgendaView;
use crate::views::day_view::DayView;
use crate::views::event_view::EventView;
use crate::views::month_view::MonthView;
use crate::views::week_view::WeekView;
use crate::{
    configuration::{config::Config, types::calendar_view::CalendarView},
    views::feed_view::FeedView,
};
use crate::{model::calendar::Calendar, views::feed_view};

/// Type alias representing a specific day in time
pub(crate) type LocalDay = DateTime<ChronoTz>;

pub(crate) type EventsByDay = BTreeMap<NaiveDate, EventList>;

pub(crate) static TEMPLATE_DIR: Dir = include_dir!("templates");
pub(crate) static ASSETS_DIR: Dir = include_dir!("assets");

#[derive(Debug)]
pub struct CalendarCollection {
    calendars: Vec<Calendar>,
    /// Events grouped by day in the display timezone
    pub(crate) events_by_day: EventsByDay,

    pub(crate) tera: Tera,
    pub(crate) config: Config,
    unparsed_properties: UnparsedProperties,
    pub(crate) cal_start: DateTime<ChronoTz>,
    pub(crate) cal_end: DateTime<ChronoTz>,
    today_date: NaiveDate,
    embed_in_page: Option<String>,
}

impl CalendarCollection {
    pub fn new(config: Config) -> eyre::Result<CalendarCollection> {
        // perform validations and transformations on the config object
        config
            .cache_timeout_duration
            .set(
                Duration::from_std(
                    parse_duration(&config.cache_timeout)
                        .wrap_err("could not parse the specified duration string")?,
                )
                .wrap_err("could not convert standard duration into Chrono::Duration")?,
            )
            .map_err(|e| eyre!(e))
            .wrap_err("could not set cache_timeout_duration")?;

        // turn the user provided "today" date into an actual NaiveDate object
        // NOTE: we were having problems with the default value from Local::now() being "invalid" so we'll just parse it here and the default can be a string
        // TODO: do we need this to be adjusted by the provided timezone?
        let today_date = parse(&config.calendar_today_date).map(|d| d.date())?;
        let cal_start = &config
            .calendar_start_date
            .as_ref()
            .map(parse)
            .transpose()
            .wrap_err("could not parse calendar_start_date")?
            .map(|t| {
                t.and_local_timezone(config.display_timezone.timezone())
                    // TODO: might want to handle ambiguous timezone conversions better
                    .single()
            });
        let cal_end = &config
            .calendar_end_date
            .as_ref()
            .map(parse)
            .transpose()
            .wrap_err("could not parse calendar_end_date")?
            .map(|t| {
                t.and_local_timezone(config.display_timezone.timezone())
                    // TODO: might want to handle ambiguous timezone conversions better
                    .single()
            });

        // load the embed page if it has been specified
        let embed_in_page = if let Some(page) = &config.embed_in_page {
            let mut embed_file = File::open(page).wrap_err("could not open embed page")?;
            let mut embed_page = String::new();
            embed_file
                .read_to_string(&mut embed_page)
                .wrap_err("could not read embed file")?;

            Some(embed_page)
        } else {
            None
        };

        // throw an error if the default view is not enabled
        let view_and_name = match config.default_calendar_view {
            CalendarView::Month => (config.render_month, "month"),
            CalendarView::Week => (config.render_week, "week"),
            CalendarView::Day => (config.render_day, "day"),
            CalendarView::Agenda => (config.render_agenda, "agenda"),
        };
        match view_and_name {
            (false, view_name) => bail!(
                "default_view is set to {} and render_{} is set to false",
                view_name,
                view_name
            ),
            (true, _) => (),
        }

        let (mut calendars, unparsed_properties) = load_calendars(&config)?;

        let cal_start = cal_start
            .unwrap_or_else(|| Some(determine_calendar_start(&config, &calendars)))
            .unwrap();
        let cal_end = cal_end
            .unwrap_or_else(|| Some(determine_calendar_end(&config, &calendars)))
            .unwrap();
        debug!("calendar runs from {} to {}", cal_start, cal_end);

        // expand recurring events
        expand_recurring_events(&mut calendars, &cal_start, &cal_end, &config)?;

        println!("Read {} calendars:", &calendars.len());
        for calendar in &calendars {
            println!("  Calendar: {}", calendar);
        }

        let events_by_day = group_events_by_day(&calendars, &config);

        // load default tera templates
        let mut tera = load_templates(&config)?;

        // we reset the page template if we are going to be embedding our pages in existing HTML
        if config.embed_in_page.is_some() {
            tera.add_raw_template("page.html", "{% block content %}{% endblock content %}")
                .wrap_err("could not override page template with blank template")?;
        }

        Ok(CalendarCollection {
            calendars,
            events_by_day,
            tera,
            config,
            unparsed_properties,
            cal_start,
            cal_end,
            today_date,
            embed_in_page,
        })
    }

    pub fn print_unparsed_properties(&self) {
        println!(
            "The following {} properties were present but have not been parsed:",
            self.unparsed_properties.len()
        );
        for property in &self.unparsed_properties {
            println!("  {}", property);
        }
    }

    pub(crate) fn today_date(&self) -> NaiveDate {
        self.today_date
    }

    pub(crate) fn display_timezone(&self) -> &ChronoTz {
        &self.config.display_timezone
    }

    /// Get a reference to the calendar collection's calendars.
    #[must_use]
    pub fn calendars(&self) -> &Vec<Calendar> {
        &self.calendars
    }

    pub(crate) fn events(&self) -> impl Iterator<Item = &Rc<Event>> {
        self.calendars.iter().flat_map(|c| c.events())
    }

    /// Generate the template context with the values to be interpolated
    ///
    /// Returns the template context of this [`CalendarCollection`].
    #[must_use]
    pub fn template_context(&self) -> Context {
        let mut context = Context::new();
        context.insert(
            "stylesheet_path",
            &self
                .config
                .base_url_path
                .join(&*self.config.stylesheet_path),
        );
        context.insert("timezone", &self.config.display_timezone.name());

        // TODO: convert these to functions of each view class
        context.insert("render_month", &self.config.render_month);
        context.insert("render_week", &self.config.render_week);
        context.insert("render_day", &self.config.render_day);
        context.insert("render_event", &self.config.render_event);
        context.insert("render_agenda", &self.config.render_agenda);
        context.insert("render_feed", &self.config.render_feed);

        // TODO: convert these to functions of each view class
        let base_url_path: unix_path::PathBuf = self.config.base_url_path.path_buf().clone();
        context.insert("month_view_path", &base_url_path.join("month"));
        context.insert("week_view_path", &base_url_path.join("week"));
        context.insert("day_view_path", &base_url_path.join("day"));
        context.insert("event_view_path", &base_url_path.join("event"));
        context.insert("agenda_view_path", &base_url_path.join("agenda"));
        context.insert("feed_view_path", &base_url_path.join(feed_view::VIEW_PATH));

        context
    }

    /// Returns the weeks to show of this [`CalendarCollection`].
    pub fn weeks_to_show(&self) -> Result<Vec<Option<Week>>> {
        // Create a DateRule to iterate over all of the weeks this calendar should display

        // get the first week starting on the configured start of month day
        // let cal_start = self.cal_start;
        let aligned_week_start = self
            .cal_start
            .checked_sub_days(Days::new(
                self.cal_start.weekday().num_days_from_sunday().into(),
            ))
            .ok_or(eyre!("could not create the aligned week start"))?;
        // TODO: make sure that we are doing the math correctly here
        let aligned_week_end = self
            .cal_end
            .checked_add_days(Days::new(
                (7 - self.cal_end.weekday().num_days_from_sunday()).into(),
            ))
            .ok_or(eyre!("could not create the aligned week end"))?;

        // setup DateRule to iterate over weeks
        let weeks_iterator = DateRule::weekly(aligned_week_start).with_end(aligned_week_end);
        let mut weeks_to_show: Vec<Option<Week>> = vec![];
        for day in weeks_iterator.into_iter() {
            weeks_to_show.push(Some(Week::new(day, self)?))
        }
        let chained_iter = iter::once(None)
            .chain(weeks_to_show)
            .chain(iter::once(None));
        // let week_windows = chained_iter.collect::<Vec<Option<DateTime<ChronoTz>>>>();
        Ok(chained_iter.collect())
    }

    pub fn events_to_show(&self) -> Result<Vec<Option<Rc<Event>>>> {
        // TODO: decide whether we want these to have previous/next links (for now, we'll go with yes for consistency with other views)
        // chain a None to the list of weeks and a None at the end
        // this will allow us to traverse the list as windows with the first and last
        // having None as appropriate
        let chained_iter = iter::once(None)
            .chain(self.events().map(|e| Some(e.clone())))
            .chain(iter::once(None));

        Ok(chained_iter.collect())
    }

    pub fn days_to_show(&self) -> Result<Vec<Option<Day>>> {
        let days_iterator = DateRule::daily(self.cal_start).with_end(self.cal_end);
        let mut days_to_show: Vec<Option<Day>> = vec![];

        for day in days_iterator.into_iter() {
            days_to_show.push(Some(Day::new(day, &self.config.display_timezone)))
        }

        // chain a None to the list of weeks and a None at the end
        // this will allow us to traverse the list as windows with the first and last
        // having None as appropriate
        let chained_iter = iter::once(None).chain(days_to_show).chain(iter::once(None));

        Ok(chained_iter.collect())
    }

    /// Get a reference to the calendar collection's tera.
    #[must_use]
    pub fn tera(&self) -> &Tera {
        &self.tera
    }

    pub fn setup_output_dir(&self) -> Result<()> {
        debug!("setting up output directory...");

        // join the output_dir to the base_dir
        // note that if the output_dir is specified as an absolute path, it will override the base_dir
        let output_dir = self.base_dir().join(&self.config.output_dir);
        debug!("output_dir: {:?}", output_dir);

        // make the output dir if it doesn't exist
        fs::create_dir_all(&output_dir)
            .context(format!("could not create output dir: {:?}", output_dir))?;

        if self.config.no_delete {
            info!("skipping delete of output directory as instructed...")
        } else {
            // remove any files present
            info!(
                "removing contents of the output directory: {:?}",
                output_dir
            );
            delete_dir_contents(&output_dir);
        }

        if self.config.copy_stylesheet_to_output {
            let stylesheet_path = &self
                .config
                .stylesheet_path
                .strip_prefix("/")
                .wrap_err("could not strip prefix")?;
            let stylesheet_destination = output_dir.join(stylesheet_path.to_str().unwrap());
            let source_stylesheet = self.base_dir().join(&self.config.copy_stylesheet_from);

            // create the stylesheet path
            let styles_dir = stylesheet_destination.parent().ok_or(eyre!(
                "could not get the parent dir of the stylesheet_destination"
            ))?;
            debug!(
                "creating the parent dir of the stylesheet_destination: {:?}",
                styles_dir
            );
            create_dir_all(styles_dir)
                .wrap_err("could not create the parent directory of the stylesheet_destination")?;

            // determine if we need to compile sass
            debug!("source_stylesheet: {:?}", source_stylesheet);
            let compile_sass = ["sass", "scss"].contains(
                &source_stylesheet
                    .extension()
                    .ok_or(eyre!("could not get extension of source_stylesheet"))?
                    .to_str()
                    .ok_or(eyre!(
                        "could not convert extension of source_stylesheet to str"
                    ))?,
            );
            debug!("need to compile sass: {:?}", compile_sass);

            // TODO: test if stylesheet exists in assets dir, otherwise use the built-in
            if source_stylesheet.exists() {
                debug!(
                    "copying stylesheet {:?} to destination: {:?}",
                    &source_stylesheet, &stylesheet_destination
                );

                if compile_sass {
                    let css_output =
                        grass::from_path(source_stylesheet, &grass::Options::default())
                            .wrap_err("could not convert SASS to CSS")?;
                    File::create(stylesheet_destination)
                        .wrap_err("could not create stylesheet_destination file")?
                        .write_all(css_output.as_bytes())
                        .wrap_err("could not write css output to stylesheet_destination")?;
                } else {
                    fs::copy(&source_stylesheet, &stylesheet_destination).wrap_err(format!(
                        "could not copy stylesheet {:?} to destination: {:?}",
                        source_stylesheet, &stylesheet_destination
                    ))?;
                };
            } else {
                debug!(
                    "source stylesheet does not exist at path: {:?}",
                    source_stylesheet
                );
                // TODO: do we want this to be an iterator? will we ever have multiple built-in stylesheets?
                for stylesheet in ASSETS_DIR.find("statical.sass")? {
                    if let FileEnt(f) = stylesheet {
                        // TODO: remove the query for the name unless we want to support multiple stylesheets
                        if let (Some(stylesheet_name), Some(stylesheet_contents)) =
                            (f.path().to_str(), f.contents_utf8())
                        {
                            debug!(
                                "copying built-in stylesheet {} to destination: {:?}",
                                stylesheet_name, &stylesheet_destination
                            );
                            let mut file = File::create(&stylesheet_destination)
                                .wrap_err("could not create destination stylesheet file")?;
                            let css_output =
                                grass::from_string(stylesheet_contents, &grass::Options::default())
                                    .wrap_err("could not convert built-in SASS to CSS")?;
                            match file.write_all(css_output.as_bytes()) {
                                Ok(_) => info!("created file from built-in stylesheet"),
                                Err(e) => error!(
                                    "could not write file to {:?}: {}",
                                    stylesheet_destination, e
                                ),
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn create_view_files(&self) -> Result<()> {
        self.setup_output_dir()?;

        // add events to views
        if self.config.render_month {
            MonthView::new(self).create_html_pages()?;
        };

        if self.config.render_week {
            WeekView::new(self).create_html_pages()?;
        };

        if self.config.render_day {
            DayView::new(self).create_html_pages()?;
        };

        if self.config.render_agenda {
            AgendaView::new(self).create_html_pages()?;
        };

        if self.config.render_event {
            EventView::new(self).create_html_pages()?;
        };

        if self.config.render_feed {
            FeedView::new(self).create_view_files()?;
        };

        Ok(())
    }

    /// Writes the selected template, with the provided context, to the provided relative file path.
    ///
    /// Note that the provided file path is appended to the base directory of this [`CalendarCollection`].
    pub fn write_template(
        &self,
        template_name: &str,
        context: &tera::Context,
        relative_file_path: &Path,
    ) -> eyre::Result<()> {
        // adjust the file path with regard to the base_directory
        let file_path = &self.base_dir().join(relative_file_path);

        // get the embed_page

        // TODO replace this with a debug or log message
        eprintln!("Writing template to file: {:?}", file_path);
        let tera_output = self.tera.render(template_name, context)?;

        let output = if let Some(page) = &self.embed_in_page {
            rewrite_str(
                page,
                Settings {
                    element_content_handlers: vec![
                        element!(&self.config.embed_element_selector, |el| {
                            el.set_inner_content(&tera_output, ContentType::Html);
                            Ok(())
                        }),
                        element!("title", |el| {
                            el.set_inner_content(
                                context
                                    .get("page_title")
                                    .expect("could not get page title from context")
                                    .as_str()
                                    .expect("could not get page title as string"),
                                ContentType::Text,
                            );
                            Ok(())
                        }),
                        element!("head", |el| {
                            el.append(
                                &(r#"<link rel="stylesheet" href=""#.to_owned()
                                    + self
                                        .config
                                        .stylesheet_path
                                        .to_str()
                                        .expect("could not get stylesheet path from config")
                                    + r#"">"#),
                                ContentType::Html,
                            );
                            Ok(())
                        }),
                    ],
                    ..Default::default()
                },
            )?
        } else {
            tera_output
        };

        // write output to file
        let mut output_file =
            File::create(file_path).wrap_err("could not create template output file")?;
        output_file
            .write_all(output.as_bytes())
            .wrap_err("could not write to template output file")?;

        Ok(())
    }

    pub(crate) fn base_dir(&self) -> &Path {
        &self.config.base_dir
    }
}

fn expand_recurring_events(
    calendars: &mut [Calendar],
    cal_start: &DateTime<ChronoTz>,
    cal_end: &DateTime<ChronoTz>,
    config: &Config,
) -> Result<(), eyre::Error> {
    log::debug!("expanding recurring events...");
    for calendar in calendars.iter_mut() {
        let pre_expansion_count = calendar.events().len();
        calendar.expand_recurrences(*cal_start, *cal_end, &config.display_timezone)?;
        log::debug!(
            "calendar events pre_expansion_count: {} post_expansion_count: {}",
            pre_expansion_count,
            calendar.events().len()
        );
    }

    Ok(())
}

#[must_use = "the loaded calendars must be stored somewhere"]
fn load_calendars(config: &Config) -> Result<(Vec<Calendar>, HashSet<String>)> {
    let mut calendars = Vec::new();
    let unparsed_properties = HashSet::new();

    // convert the CalendarSourceConfigs into Result<CalendarSources>
    debug!("configuring calendar sources...");
    let mut calendars_sources_configs: Vec<Result<CalendarSource>> = Vec::new();
    for source_config in &config.calendar_sources {
        debug!("creating calendar source: {:?}", &source_config);
        calendars_sources_configs.push(CalendarSource::new(
            &config.base_dir,
            source_config.clone(),
            config,
        ));
    }

    // sort properly configured calendars and errors
    let (calendar_sources, calendar_errors): (
        Vec<Result<CalendarSource>>,
        Vec<Result<CalendarSource>>,
    ) = calendars_sources_configs
        .into_iter()
        .partition(|s| s.is_ok());
    debug!(
        "{} valid and {} erroneous calendar sources",
        calendar_sources.len(),
        calendar_errors.len()
    );

    // bail if any of them failed
    if !calendar_errors.is_empty() {
        // TODO: let the user configure whether to bail or just report errors and continue
        bail!("errors in calendars configuration")
    }

    // check after we check for error conditions so we don't hide errors behind a false empty condition
    if calendar_sources.is_empty() {
        bail!("no valid calendar sources found");
    }

    // parse calendar sources that are ok
    debug!("parsing calendars...");
    for source in calendar_sources.into_iter().flatten() {
        debug!("parsing calendar source: {:?}", source);
        match source.parse_calendars(config) {
            Ok(mut parsed_calendars) => {
                calendars.append(&mut parsed_calendars);
            }
            Err(e) => {
                error!("could not parse source: {:?}", e);
            }
        }
    }

    Ok((calendars, unparsed_properties))
}

#[must_use]
fn determine_calendar_start(config: &Config, calendars: &[Calendar]) -> DateTime<ChronoTz> {
    // get start date for entire collection
    calendars
        .iter()
        .map(|c| c.start().with_timezone(&config.display_timezone.into()))
        .reduce(|min_start, start| min_start.min(start))
        .unwrap_or_else(|| Utc::now().with_timezone(&config.display_timezone.into()))
}

#[must_use]
fn determine_calendar_end(config: &Config, calendars: &[Calendar]) -> DateTime<ChronoTz> {
    let end_of_month_default =
        DateRule::monthly(Utc::now().with_timezone(&config.display_timezone.into()))
            .with_rolling_day(31)
            .unwrap()
            .next()
            .expect("could not get end of month");

    // get end date for entire collection
    calendars
        .iter()
        .map(|c| c.end().with_timezone(&config.display_timezone.into()))
        .reduce(|max_end, end| max_end.max(end))
        .unwrap_or(end_of_month_default)
}

#[must_use]
fn group_events_by_day(
    calendars: &[Calendar],
    config: &Config,
) -> BTreeMap<NaiveDate, Vec<Rc<Event>>> {
    // TODO might want to hand back a better event collection e.g. might want to de-duplicate them
    let mut events_by_day = EventsByDay::new();

    for (event_num, event) in calendars.iter().flat_map(|c| c.events()).enumerate() {
        // TODO: find out if event is longer than 1 day
        // TODO: find out if the event crosses a day boundary in this timezone
        // TODO: find out if this event ends on this day
        let event_days = event.days_with_timezone(&config.display_timezone);
        println!(
            "Event {} (day span: {})\n  {}",
            event_num,
            event_days.len(),
            event
        );
        for day in event_days {
            events_by_day
                // TODO: do we need to adjust for timezone here?
                .entry(
                    day.with_timezone::<chrono_tz::Tz>(&config.display_timezone.into())
                        .date_naive(),
                )
                .or_default()
                .push(event.clone());
        }
    }

    events_by_day
}

#[must_use = "the loaded templates must be stored somewhere"]
fn load_templates(config: &Config) -> Result<Tera, eyre::Error> {
    info!("loading default templates...");
    let mut tera = Tera::default();
    let default_templates = TEMPLATE_DIR.find("**/*.html")?.filter_map(|t| match t {
        DirEnt(_) => None,
        FileEnt(t) => Some((
            t.path()
                .to_str()
                .expect("could not get default template name"),
            t.contents_utf8()
                .expect("could not get default template contents"),
        )),
    });

    tera.add_raw_templates(default_templates)
        .wrap_err("could not add default templates to Tera")?;

    info!("loading custom templates...");
    let custom_templates: Vec<(PathBuf, Option<String>)> = config
        .base_dir
        // we're joining with base_dir here to ensure that the templates are found relative to the config file
        .join(&config.template_path)
        .read_dir()
        .wrap_err("could not read custom templates dir")?
        .filter_map_ok(|t| Some(t.path()))
        .map(|t| (t.unwrap(), None))
        .collect();
    tera.add_template_files(custom_templates)
        .wrap_err("could not add custom templates")?;

    Ok(tera)
}
