use color_eyre::eyre::Result;
use std::{fs::File, path::PathBuf, rc::Rc};
use tera::{Context, Tera};
use time_tz::TimeZone;

use crate::{
    config::{CalendarView, ParsedConfig},
    model::event::Event,
    util::render_to,
};

#[derive(Debug)]
pub(crate) struct AgendaView {
    /// The output directory for agenda view files
    output_dir: PathBuf,
    event_list: Vec<Rc<Event>>,
}

impl AgendaView {
    pub fn new(output_dir: PathBuf) -> Self {
        let event_list = Vec::new();
        AgendaView {
            output_dir,
            event_list,
        }
    }

    pub fn add_event(&mut self, event: &Rc<Event>) {
        // TODO could sort events into past and future here
        self.event_list.push(event.clone())
    }

    pub fn create_html_pages(&self, config: &ParsedConfig, tera: &Tera) -> Result<()> {
        let mut previous_file_name: Option<String> = None;
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

        // create a peekable iterator
        let mut agenda_events = past_events.iter().peekable();

        while let Some((events, page)) = agenda_events.next() {
            println!("page {}", page);
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

            let events: Vec<_> = events
                .iter()
                .map(|e| e.context(config.display_timezone))
                .collect();

            let file_name = format!("{}.html", page);

            let next_page_opt = agenda_events.peek();
            let next_file_name = next_page_opt.map(|(_, next_page)| format!("{}.html", next_page));

            println!(
                "  {:?} {:?} {:?}",
                previous_file_name, file_name, next_file_name
            );

            let mut template_out_file = self.output_dir.join(PathBuf::from(&file_name));

            let mut context = Context::new();
            context.insert("stylesheet_path", &config.stylesheet_path);
            context.insert("timezone", config.display_timezone.name());
            context.insert("page", &page);
            context.insert("events", &events);
            context.insert("previous_file_name", &previous_file_name);
            context.insert("next_file_name", &next_file_name);
            println!("Writing template to file: {:?}", template_out_file);
            render_to(
                tera,
                "agenda.html",
                &context,
                File::create(&template_out_file)?,
            )?;

            // write the index page for the current week
            // TODO might want to write the index if next_week is None and nothing has been written yet
            if let Some(next_page) = next_page_opt {
                if !index_written {
                    let (_events, page) = next_page;
                    // write the index file if the next month is after the current date
                    // TODO make sure that the conditional tests are correct, maybe add some tests
                    // TODO handle the case when there is no page 1 (when there are less than agenda_events_per_page past current)
                    if page == &1_isize {
                        template_out_file.pop();
                        template_out_file.push(PathBuf::from("index.html"));

                        println!("Writing template to index file: {:?}", template_out_file);
                        render_to(
                            tera,
                            "agenda.html",
                            &context,
                            File::create(&template_out_file)?,
                        )?;
                        index_written = true;

                        // write the main index as the week view
                        if config.default_calendar_view == CalendarView::Agenda {
                            template_out_file.pop();
                            template_out_file.pop();
                            template_out_file.push(PathBuf::from("index.html"));
                            println!(
                                "Writing template to main index file: {:?}",
                                template_out_file
                            );
                            render_to(
                                tera,
                                "agenda.html",
                                &context,
                                File::create(template_out_file)?,
                            )?;
                        }
                    }
                }
            }

            previous_file_name = Some(file_name);
        }

        Ok(())
    }
}
