use color_eyre::eyre::{bail, Context, Result};
use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};
use url::Url;

use crate::{
    configuration::calendar_source_config::CalendarSourceConfig, model::calendar::Calendar,
};

#[derive(Debug)]
pub(crate) enum CalendarSource<'a> {
    CalendarUrl(Url, &'a CalendarSourceConfig),
    CalendarFile(PathBuf, &'a CalendarSourceConfig),
}

impl CalendarSource<'_> {
    pub(crate) fn new<'a>(
        base_dir: &'a Path,
        source_config: &'a CalendarSourceConfig,
    ) -> Result<CalendarSource<'a>> {
        log::debug!("creating calendar source: {}", source_config);
        if let Ok(url) = Url::parse(source_config.into()) {
            log::debug!("calendar source is a url");
            return Ok(CalendarSource::CalendarUrl(url, source_config));
        };

        let path = base_dir.join(
            PathBuf::try_from(&source_config)
                .wrap_err("calendar source is not a valid file path")?,
        );

        if path.exists() {
            log::debug!("calendar source is a file that exists");
            Ok(CalendarSource::CalendarFile(path, source_config))
        } else {
            bail!("could not create CalendarSource from: {}", source_config);
        }
    }

    /// Returns the parsed calendars of this [`CalendarSource`].
    ///
    /// Listed as plural because a single source may contain multiple calendars as per the ical/ics standard.
    pub(crate) fn parse_calendars(&self, base_dir: &Path) -> Result<Vec<Calendar>> {
        let parsed_calendars = match self {
            Self::CalendarFile(file, source_config) => {
                log::info!("reading calendar file: {:?}", file);
                let buf = BufReader::new(File::open(base_dir.join(file))?);
                Calendar::parse_calendars(buf, source_config)?
            }
            Self::CalendarUrl(url, source_config) => {
                log::info!("reading calendar url: {}", url);
                let ics_string = ureq::get(url.as_ref()).call()?.into_string()?;
                Calendar::parse_calendars(ics_string.as_bytes(), source_config)?
            }
        };

        Ok(parsed_calendars)
    }
}
