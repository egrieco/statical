use color_eyre::eyre::Result;
use std::{
    collections::BTreeMap,
    isize, iter,
    path::{Path, PathBuf},
    rc::Rc,
};
use tera::{Context, Tera};
use time::macros::format_description;
use time_tz::TimeZone;

use crate::{
    config::{CalendarView, ParsedConfig},
    model::{
        calendar::Calendar,
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
pub(crate) struct AgendaView {
    /// The output directory for agenda view files
    output_dir: PathBuf,
    event_list: Vec<Rc<Event>>,
}

impl AgendaView {
    pub fn new(output_dir: PathBuf, calendars: &Vec<Calendar>) -> Self {
        let mut event_list = Vec::new();

        // add events to the event_list
        for calendar in calendars {
            for event in calendar.events() {
                event_list.push(event.clone())
            }
        }

        AgendaView {
            output_dir,
            event_list,
        }
    }

    pub fn create_html_pages(&self, config: &ParsedConfig, tera: &Tera) -> Result<()> {
        let mut index_written = false;

        // partition events into past and future events
        let (mut past_events, mut future_events): (Vec<_>, Vec<_>) = self
            .event_list
            .iter()
            .partition(|e| e.start().date() < config.agenda_start_date);

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
            .chain(iter::once(None).into_iter());
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
        config: &ParsedConfig,
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

        let event_contexts: Vec<_> = events
            .iter()
            .map(|e| e.context(config.display_timezone))
            .collect();

        let file_name = format!("{}.html", page);
        let previous_file_name = previous_page.map(|(page_num, _)| format!("{}.html", page_num));
        let next_file_name = next_page.map(|(page_num, _)| format!("{}.html", page_num));

        println!(
            "  {:?} {:?} {:?}",
            previous_file_name, file_name, next_file_name
        );

        let mut context = Context::new();
        context.insert("stylesheet_path", &config.stylesheet_path);
        context.insert("timezone", config.display_timezone.name());
        context.insert("page", &page);
        context.insert("events", &event_contexts);

        // group events by whatever format is specified
        // TODO add agenda group format to the config file
        let mut event_groups = EventDayGroups::new();
        for event in events.iter() {
            event_groups
                .entry(event.start().format(format_description!(
                    "[weekday repr:short], [day] [month repr:short] [year]"
                ))?)
                .or_default()
                .push(event.context(config.display_timezone))
        }
        context.insert("event_groups", &event_groups);

        // create the main file path
        let binding = output_dir.join(PathBuf::from(&file_name));
        let mut file_paths = vec![&binding];
        // then add any additional index paths
        file_paths.extend(index_paths);

        // write the template to all specified paths
        for file_path in file_paths {
            // if the path matches the root path, prepend the default view to the next and previous links
            if file_path.parent() == Some(&config.output_dir) {
                context.insert(
                    "previous_file_name",
                    &previous_file_name
                        .as_ref()
                        .map(|path| ["agenda", path].join("/")),
                );
                context.insert(
                    "next_file_name",
                    &next_file_name
                        .as_ref()
                        .map(|path| ["agenda", path].join("/")),
                );
            } else {
                context.insert("previous_file_name", &previous_file_name);
                context.insert("next_file_name", &next_file_name);
            }

            // write the actual template
            write_template(tera, "agenda.html", &context, file_path)?;
        }

        Ok(())
    }
}
