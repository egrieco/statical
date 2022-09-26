use color_eyre::eyre::Result;
use std::{collections::BTreeMap, fs::File, path::PathBuf, rc::Rc};
use tera::{Context, Tera};
use time_tz::TimeZone;

use super::week_view::WeekMap;
use crate::{
    config::{CalendarView, ParsedConfig},
    model::{
        calendar_collection::{
            blank_context, iso_weeks_for_month_display, month_from_u8, WeekContext,
        },
        event::{Event, WeekNum, Year},
    },
    util::{self, render_to},
    views::week_view::WeekDayMap,
};

/// Type alias representing a specific month in time
type Month = (Year, u8);

/// A BTreeMap of Vecs grouped by specific months
pub type MonthMap = BTreeMap<Month, WeekMapList>;
type WeekMapList = BTreeMap<WeekNum, WeekMap>;

#[derive(Debug)]
pub struct MonthView {
    month_map: MonthMap,
}

impl MonthView {
    pub fn new() -> Self {
        let month_map = BTreeMap::new();
        MonthView { month_map }
    }

    pub fn add_event(&mut self, event: &Rc<Event>) {
        self.month_map
            .entry((event.year(), event.start().month() as u8))
            .or_insert(WeekMapList::new())
            .entry(event.week())
            .or_insert(WeekMap::new())
            .entry((event.year(), event.week()))
            .or_insert(Vec::new())
            .push(event.clone());
    }

    pub fn create_html_pages(&self, config: &ParsedConfig, tera: &Tera) -> Result<()> {
        let output_dir = util::create_subdir(&config.output_dir, "month")?;

        let mut previous_file_name: Option<String> = None;
        let mut index_written = false;

        let mut months_iter = self.month_map.iter().peekable();
        while let Some(((year, month), weeks)) = months_iter.next() {
            println!("month: {}", month);
            let mut week_list = Vec::new();

            // create all weeks in this month
            let weeks_for_display = iso_weeks_for_month_display(year, month)?;
            println!("From week {:?}", weeks_for_display);
            for week_num in weeks_for_display {
                match weeks.get(&week_num) {
                    Some(week_map) => {
                        println!("  Creating week {}, {} {}", week_num, month, year);
                        for ((_y, _w), events) in week_map {
                            let mut week_day_map: WeekDayMap = BTreeMap::new();

                            for event in events {
                                println!(
                                    "    event: ({} {} {}) {} {}",
                                    event.start().weekday(),
                                    event.year(),
                                    event.week(),
                                    event.summary(),
                                    event.start(),
                                );
                                let day_of_week = event.start().weekday().number_days_from_sunday();
                                week_day_map
                                    .entry(day_of_week)
                                    .or_insert(Vec::new())
                                    .push(event.clone());
                            }

                            // create week days
                            let week_dates =
                                week_day_map.context(year, &week_num, config.display_timezone)?;
                            week_list.push(week_dates);
                        }
                    }
                    None => {
                        println!("  Inserting blank week {}, {} {}", week_num, month, year);
                        week_list.push(blank_context(year, &week_num)?);
                    }
                }
            }

            let file_name = format!("{}-{}.html", year, month);
            let next_month = months_iter.peek();
            let next_file_name = next_month.map(|((next_year, next_month), _events)| {
                format!("{}-{}.html", next_year, next_month)
            });
            let mut template_out_file = output_dir.join(PathBuf::from(&file_name));

            let mut context = Context::new();
            context.insert("stylesheet_path", &config.stylesheet_path);
            context.insert("timezone", &config.display_timezone.name());
            context.insert("year", &year);
            context.insert("month", &month);
            context.insert("weeks", &week_list);
            context.insert("previous_file_name", &previous_file_name);
            context.insert("next_file_name", &next_file_name);
            println!("Writing template to file: {:?}", template_out_file);
            render_to(
                tera,
                "month.html",
                &context,
                File::create(&template_out_file)?,
            )?;

            // write the index page for the current month
            if !index_written {
                if let Some(next_month_num) =
                    next_month.map(|((_next_year, next_month), _events)| next_month)
                {
                    // write the index file if the next month is after the current date
                    if month_from_u8(*next_month_num)? as u8
                        > config.agenda_start_date.month() as u8
                    {
                        template_out_file.pop();
                        template_out_file.push(PathBuf::from("index.html"));

                        println!("Writing template to index file: {:?}", template_out_file);
                        render_to(
                            tera,
                            "month.html",
                            &context,
                            File::create(&template_out_file)?,
                        )?;
                        index_written = true;

                        // write the main index as the month view
                        if config.default_calendar_view == CalendarView::Month {
                            template_out_file.pop();
                            template_out_file.pop();
                            template_out_file.push(PathBuf::from("index.html"));
                            println!(
                                "Writing template to main index file: {:?}",
                                template_out_file
                            );
                            render_to(
                                tera,
                                "month.html",
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
