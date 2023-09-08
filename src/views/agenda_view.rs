use chrono::Datelike;
use color_eyre::eyre::Result;
use std::{
    collections::BTreeMap,
    isize, iter,
    path::{Path, PathBuf},
    rc::Rc,
};
use tera::{Context, Tera};

use crate::{
    config::{CalendarView, Config},
    model::{
        calendar_collection::CalendarCollection,
        event::{Event, EventContext},
    },
    util::write_template,
};

type AgendaPageId = isize;
type EventSlice<'a> = &'a [&'a Rc<Event>];

/// A triple with the previous, current, and next agenda pages present
///
/// Note that the previous and next weeks may be None
pub type AgendaSlice<'a> = &'a [Option<(&'a AgendaPageId, &'a EventSlice<'a>)>];

type EventDayGroups = BTreeMap<String, Vec<EventContext>>;

#[derive(Debug)]
pub(crate) struct AgendaView<'a> {
    /// The output directory for agenda view files
    output_dir: PathBuf,
    calendars: &'a CalendarCollection,
}

impl AgendaView<'_> {
    pub fn new(output_dir: PathBuf, calendars: &CalendarCollection) -> AgendaView<'_> {
        AgendaView {
            output_dir,
            calendars,
        }
    }

    fn event_list(&self) -> impl Iterator<Item = &Rc<Event>> {
        self.calendars.events()
    }

    pub fn create_html_pages(&self, config: &Config, tera: &Tera) -> Result<()> {
        let mut index_written = false;

        // partition events into past and future events
        let (mut past_events, mut future_events): (Vec<_>, Vec<_>) = self
            .event_list()
            .partition(|e| e.start() < config.calendar_today_date);

        // process past events
        past_events.sort_by_key(|e| e.start());
        let mut past_events: Vec<_> = past_events
            .rchunks(config.agenda_events_per_page)
            .zip((1_isize..).map(|i| -i))
            .collect();
        past_events.reverse();

        // process future events
        future_events.sort_by_key(|e| e.start());
        let future_events_iter = future_events.chunks(config.agenda_events_per_page).zip(0..);

        // combine all events into one list
        past_events.extend(future_events_iter);

        // let event_pages = past_events
        //     .into_iter()
        //     .map(|(events, page)| (page, events))
        //     .collect();

        // chain a None to the list of agenda blocks and a None at the end
        // this will allow us to traverse the list as windows with the first and last
        // having None as appropriate
        let chained_iter = iter::once(None)
            .chain(
                past_events
                    .iter()
                    .map(|(events, page)| Some((page, events))),
            )
            .chain(iter::once(None));
        let page_windows = &chained_iter.collect::<Vec<Option<(&AgendaPageId, &EventSlice)>>>();

        // iterate through all windows
        for window in page_windows.windows(3) {
            let next_page_opt = window[2];

            let mut index_paths = vec![];

            // write the index page for the current week
            // TODO might want to write the index if next_week is None and nothing has been written yet
            if !index_written {
                if let Some(next_page) = next_page_opt {
                    let (page, _events) = next_page;
                    // write the index file if the next month is after the current date
                    // TODO make sure that the conditional tests are correct, maybe add some tests
                    // TODO handle the case when there is no page 1 (when there are less than agenda_events_per_page past current)
                    if page == &1_isize {
                        index_written = true;
                        index_paths.push(self.output_dir.join(PathBuf::from("index.html")));

                        // write the main index as the week view
                        if config.default_calendar_view == CalendarView::Agenda {
                            index_paths.push(config.output_dir.join(PathBuf::from("index.html")));
                        }
                    }
                } else {
                    index_written = true;
                    index_paths.push(self.output_dir.join(PathBuf::from("index.html")));

                    // write the main index as the week view
                    if config.default_calendar_view == CalendarView::Agenda {
                        index_paths.push(config.output_dir.join(PathBuf::from("index.html")));
                    }
                }
            }

            self.write_view(
                config,
                tera,
                &window,
                &self.output_dir,
                index_paths.as_slice(),
            )?;
        }

        Ok(())
    }

    /// Takes a `AgendaSlice` and writes the corresponding file
    ///
    /// # Panics
    ///
    /// Panics if the current_page (in the middle of the slice) is ever None. This should never happen.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file cannot be written to disk.
    fn write_view(
        &self,
        config: &Config,
        tera: &Tera,
        agenda_slice: &AgendaSlice,
        output_dir: &Path,
        index_paths: &[PathBuf],
    ) -> Result<()> {
        let previous_page = agenda_slice[0];
        let current_page =
            agenda_slice[1].expect("Current agenda page is None. This should never happen.");
        let next_page = agenda_slice[2];

        let (page, events) = current_page;

        println!("page {:?}", page);
        for event in events.iter() {
            println!(
                "  event: ({} {} {}) {} {}",
                event.start().weekday(),
                event.year(),
                event.week(),
                event.summary(),
                event.start(),
            );
        }

        let event_contexts: Vec<_> = events.iter().map(|e| e.context(config)).collect();

        let file_name = format!("{}.html", page);
        let previous_file_name = previous_page.map(|(page_num, _)| format!("{}.html", page_num));
        let next_file_name = next_page.map(|(page_num, _)| format!("{}.html", page_num));

        println!(
            "  {:?} {:?} {:?}",
            previous_file_name, file_name, next_file_name
        );

        let mut context = Context::new();
        context.insert(
            "stylesheet_path",
            &config.base_url_path.join(&*config.stylesheet_path),
        );
        context.insert("timezone", config.display_timezone.name());
        // TODO: we need to refactor the way agenda pages are created before we can enable the below
        // context.insert(
        //     "view_date_start",
        //     &current_page
        //         .format(&config.agenda_view_format_start)
        //         .to_string(),
        // );
        // context.insert(
        //     "view_date_end",
        //     &current_page
        //         .format(&config.agenda_view_format_end)
        //         .to_string(),
        // );
        context.insert("page", &page);
        context.insert("events", &event_contexts);

        // group events by whatever format is specified
        // TODO add agenda group format to the config file
        let mut event_groups = EventDayGroups::new();
        for event in events.iter() {
            event_groups
                .entry(event.start().format("%a, %d %m %Y").to_string())
                .or_default()
                .push(event.context(config))
        }
        context.insert("event_groups", &event_groups);

        let base_url_path: unix_path::PathBuf =
            self.calendars.config.base_url_path.path_buf().clone();
        context.insert("month_view_path", &base_url_path.join("month"));
        context.insert("week_view_path", &base_url_path.join("week"));
        context.insert("day_view_path", &base_url_path.join("day"));
        context.insert("agenda_view_path", &base_url_path.join("agenda"));

        // create the main file path
        let binding = output_dir.join(PathBuf::from(&file_name));
        let mut file_paths = vec![&binding];
        // then add any additional index paths
        file_paths.extend(index_paths);

        // write the template to all specified paths
        for file_path in file_paths {
            let view_path = base_url_path.join("agenda");
            context.insert(
                "previous_file_name",
                &previous_file_name.as_ref().map(|path| view_path.join(path)),
            );
            context.insert(
                "next_file_name",
                &next_file_name.as_ref().map(|path| view_path.join(path)),
            );

            // write the actual template
            write_template(tera, "agenda.html", &context, file_path)?;
        }

        Ok(())
    }
}
