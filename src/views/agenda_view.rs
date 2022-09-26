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

        // partition events into past and future events
        let (past_events, future_events): (Vec<_>, Vec<_>) = self
            .event_list
            .iter()
            .partition(|e| e.start().date() < config.agenda_start_date);

        // process past events
        let mut past_events_iter = past_events
            .rchunks(config.agenda_events_per_page)
            .zip(1_isize..)
            .peekable();
        while let Some((events, page)) = past_events_iter.next() {
            println!("page: {}", page);
            for event in events {
                println!(
                    "  event: ({} {} {}) {} {}",
                    event.start().weekday(),
                    event.year(),
                    event.week(),
                    event.summary(),
                    event.start(),
                );
            }
            let file_name = format!("{}.html", -page);
            let previous_file_name = past_events_iter
                .peek()
                .map(|(_, previous_page)| format!("{}.html", -previous_page));
            let next_file_name = format!("{}.html", 1 - page);

            let template_out_file = output_dir.join(PathBuf::from(&file_name));

            let mut context = Context::new();
            context.insert("stylesheet_path", &config.stylesheet_path);
            context.insert("timezone", config.display_timezone.name());
            context.insert("page", &page);
            context.insert("events", events);
            context.insert("previous_file_name", &previous_file_name);
            context.insert("next_file_name", &next_file_name);
            println!("Writing template to file: {:?}", template_out_file);
            render_to(
                tera,
                "agenda.html",
                &context,
                File::create(template_out_file)?,
            )?;
        }

        // process future events
        if future_events.is_empty() {
            println!("page: 0");
            let previous_file_name = if past_events.is_empty() {
                None
            } else {
                Some("-1.html")
            };

            let template_out_file = &output_dir.join(PathBuf::from("0.html"));

            let mut context = Context::new();
            context.insert("stylesheet_path", &config.stylesheet_path);
            context.insert("timezone", config.display_timezone.name());
            context.insert("page", &0);
            context.insert("events", &future_events);
            context.insert("previous_file_name", &previous_file_name);
            context.insert("next_file_name", &None::<&str>);
            println!("Writing template to file: {:?}", template_out_file);
            render_to(
                tera,
                "agenda.html",
                &context,
                File::create(template_out_file)?,
            )?;
        } else {
            let mut future_events_iter = future_events
                .rchunks(config.agenda_events_per_page)
                .zip(0..)
                .peekable();
            while let Some((events, page)) = future_events_iter.next() {
                println!("page: {}", page);
                for event in events {
                    println!(
                        "  event: ({} {} {}) {} {}",
                        event.start().weekday(),
                        event.year(),
                        event.week(),
                        event.summary(),
                        event.start(),
                    );
                }
                let file_name = format!("{}.html", page);
                let previous_file_name = if page == 0 && past_events.is_empty() {
                    None
                } else {
                    Some(format!("{}.html", page - 1))
                };
                let next_file_name = future_events_iter
                    .peek()
                    .map(|(_, next_page)| format!("{}.html", next_page));

                let template_out_file = output_dir.join(PathBuf::from(&file_name));

                let mut context = Context::new();
                context.insert("stylesheet_path", &config.stylesheet_path);
                context.insert("timezone", config.display_timezone.name());
                context.insert("page", &page);
                context.insert("events", events);
                context.insert("previous_file_name", &previous_file_name);
                context.insert("next_file_name", &next_file_name);
                println!("Writing template to file: {:?}", template_out_file);
                render_to(
                    tera,
                    "agenda.html",
                    &context,
                    File::create(template_out_file)?,
                )?;
            }
        }

        Ok(())
    }
}
