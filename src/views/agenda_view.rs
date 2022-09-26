use color_eyre::eyre::Result;
use std::{fs::File, path::PathBuf, rc::Rc};
use tera::{Context, Tera};
use time_tz::TimeZone;

use crate::{
    config::ParsedConfig,
    model::event::Event,
    util::{self, render_to},
};

#[derive(Debug)]
pub(crate) struct AgendaView {
    event_list: Vec<Rc<Event>>,
}

impl Default for AgendaView {
    fn default() -> Self {
        Self::new()
    }
}

impl AgendaView {
    pub fn new() -> Self {
        let event_list = Vec::new();
        AgendaView { event_list }
    }

    pub fn add_event(&mut self, event: &Rc<Event>) {
        // TODO could sort events into past and future here
        self.event_list.push(event.clone())
    }

    pub fn create_html_pages(&self, config: &ParsedConfig, tera: &Tera) -> Result<()> {
        let output_dir = util::create_subdir(&config.output_dir, "agenda")?;

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
                .into_iter()
                .map(|e| e.context(config.display_timezone))
                .collect();

            let file_name = format!("{}.html", page);

            let next_file_name = agenda_events
                .peek()
                .map(|(_, next_page)| format!("{}.html", next_page));

            println!(
                "  {:?} {:?} {:?}",
                previous_file_name, file_name, next_file_name
            );

            let template_out_file = output_dir.join(PathBuf::from(&file_name));

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
                File::create(template_out_file)?,
            )?;

            // TODO add index file writing here

            previous_file_name = Some(file_name);
        }

        Ok(())
    }
}
