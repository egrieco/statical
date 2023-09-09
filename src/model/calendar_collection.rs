use chrono::{DateTime, Datelike, Days, NaiveDate, Utc};
use chrono_tz::Tz as ChronoTz;
use chronoutil::DateRule;
use color_eyre::eyre::{self, bail, eyre, Context as EyreContext, Result};
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use include_dir::{
    include_dir, Dir,
    DirEntry::{Dir as DirEnt, File as FileEnt},
};
use log::{debug, info};
use std::collections::{BTreeMap, HashSet};
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::rc::Rc;
use std::{fs, iter};
use tera::{Context, Tera};

use super::calendar_source::CalendarSource;
use super::day::Day;
use super::event::{Event, EventList, UnparsedProperties};
use super::week::Week;
use crate::configuration::{config::Config, options::Opt};
use crate::model::calendar::Calendar;
use crate::util::delete_dir_contents;
use crate::views::agenda_view::AgendaView;
use crate::views::day_view::DayView;
use crate::views::month_view::MonthView;
use crate::views::week_view::WeekView;

/// Type alias representing a specific day in time
pub(crate) type LocalDay = DateTime<ChronoTz>;

pub(crate) type EventsByDay = BTreeMap<NaiveDate, EventList>;

static TEMPLATE_DIR: Dir = include_dir!("templates");

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
}

impl CalendarCollection {
    pub fn new(args: Opt) -> eyre::Result<CalendarCollection> {
        log::info!("reading configuration...");
        let config: Config = Figment::from(Serialized::defaults(Config::default()))
            .merge(Toml::file("statical.toml"))
            .admerge(Serialized::defaults(args))
            .extract()?;

        let mut calendars = Vec::new();
        let mut unparsed_properties = HashSet::new();

        // convert the CalendarSourceConfigs into Result<CalendarSources>
        let calendars_sources_configs = config.calendar_sources.iter().map(CalendarSource::new);

        // sort properly configured calendars and errors
        let (calendar_sources, calendar_errors): (
            Vec<Result<CalendarSource>>,
            Vec<Result<CalendarSource>>,
        ) = calendars_sources_configs.partition(|s| s.is_ok());

        // bail if any of them failed
        if !calendar_errors.is_empty() {
            // TODO: let the user configure whether to bail or just report errors and continue
            bail!("errors in calendars configuration")
        }

        // parse calendar sources that are ok
        for source in calendar_sources {
            match source?.parse_calendars() {
                Ok((mut parsed_calendars, calendar_unparsed_properties)) => {
                    unparsed_properties.extend(calendar_unparsed_properties.clone().into_iter());
                    calendars.append(&mut parsed_calendars);
                }
                Err(_) => todo!(),
            }
        }

        // TODO: have each calendar determine its own start and end
        let end_of_month_default =
            DateRule::monthly(Utc::now().with_timezone(&config.display_timezone.into()))
                .with_rolling_day(31)
                .unwrap()
                .next()
                .unwrap();
        // .ok_or(eyre!("could not get end of month")?;

        // get start and end date for entire collection
        let cal_start = calendars
            .iter()
            .map(|c| c.start().with_timezone(&config.display_timezone.into()))
            .reduce(|min_start, start| min_start.min(start))
            .unwrap_or_else(|| Utc::now().with_timezone(&config.display_timezone.into()));
        let cal_end = calendars
            .iter()
            .map(|c| c.end().with_timezone(&config.display_timezone.into()))
            .reduce(|max_end, end| max_end.max(end))
            // TODO consider a better approach to finding the correct number of days
            .unwrap_or(end_of_month_default);

        // expand recurring events
        // TODO: make Events an enum so that original events and recurrences are distinct
        log::debug!("expanding recurring events...");
        for calendar in calendars.iter_mut() {
            let pre_expansion_count = calendar.events().len();
            calendar.expand_recurrences(cal_start, cal_end, &config.display_timezone)?;
            log::debug!(
                "calendar events pre_expansion_count: {} post_expansion_count: {}",
                pre_expansion_count,
                calendar.events().len()
            );
        }

        println!("Read {} calendars:", &calendars.len());
        for calendar in &calendars {
            println!("  Calendar: {}", calendar);
        }

        // TODO might want to hand back a better event collection e.g. might want to de-duplicate them
        let mut events_by_day = EventsByDay::new();

        for (event_num, event) in calendars.iter().flat_map(|c| c.events()).enumerate() {
            // find out if event is longer than 1 day
            // find out if the event crosses a day boundary in this timezone
            // find out if this event ends on this day
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

        // load custom tera templates
        info!("loading custom templates...");
        let mut tera = Tera::new("templates/**/*.html")?;

        // load default tera templates
        info!("loading default templates...");
        let mut default_templates = Tera::default();
        for template in TEMPLATE_DIR.find("**/*.html")? {
            match template {
                DirEnt(_) => Ok(()),
                FileEnt(t) => match (t.path().to_str(), t.contents_utf8()) {
                    (Some(template_name), Some(template_contents)) => {
                        debug!("adding default template: {}", template_name);
                        default_templates.add_raw_template(template_name, template_contents)
                    }
                    // TODO: probably want to surface these errors
                    (_, _) => Ok(()),
                },
            }?;
        }

        // combine the defaults with the custom templates
        tera.extend(&default_templates)?;

        Ok(CalendarCollection {
            calendars,
            events_by_day,
            tera,
            config,
            unparsed_properties,
            cal_start,
            cal_end,
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

    /// Get a reference to the calendar collection's calendars.
    #[must_use]
    pub fn calendars(&self) -> &[Calendar] {
        self.calendars.as_ref()
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

        let base_url_path: unix_path::PathBuf = self.config.base_url_path.path_buf().clone();
        context.insert("month_view_path", &base_url_path.join("month"));
        context.insert("week_view_path", &base_url_path.join("week"));
        context.insert("day_view_path", &base_url_path.join("day"));
        context.insert("agenda_view_path", &base_url_path.join("agenda"));

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
            weeks_to_show.push(Some(Week::new(day, &self.config.display_timezone)?))
        }
        let chained_iter = iter::once(None)
            .chain(weeks_to_show)
            .chain(iter::once(None));
        // let week_windows = chained_iter.collect::<Vec<Option<DateTime<ChronoTz>>>>();
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
        let output_dir = &PathBuf::from(&self.config.output_dir);

        // make the output dir if it doesn't exist
        fs::create_dir_all(output_dir)
            .context(format!("could not create output dir: {:?}", output_dir))?;

        if self.config.no_delete {
            info!("skipping delete of output directory as instructed...")
        } else {
            // remove any files present
            info!(
                "removing contents of the output directory: {:?}",
                output_dir
            );
            delete_dir_contents(output_dir);
        }

        // create the styles dir
        let styles_dir = output_dir.join("styles");
        create_dir_all(&styles_dir)?;

        if self.config.copy_stylesheet_to_output {
            let stylesheet_destination = styles_dir.join(PathBuf::from("style.css"));
            let source_stylesheet = &&self.config.copy_stylesheet_from;
            fs::copy(source_stylesheet, &stylesheet_destination).context(format!(
                "could not copy stylesheet {:?} to destination: {:?}",
                source_stylesheet, stylesheet_destination
            ))?;
        }

        Ok(())
    }

    pub fn create_html_pages(&self) -> Result<()> {
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

        Ok(())
    }
}
