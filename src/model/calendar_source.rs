use color_eyre::eyre::{bail, Context, Result};
use std::{
    collections::HashSet,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};
use url::Url;

use crate::{
    configuration::calendar_source_config::CalendarSourceConfig, model::calendar::Calendar,
};

#[derive(Debug)]
pub(crate) enum CalendarSource {
    CalendarUrl(Url),
    CalendarFile(PathBuf),
}

impl CalendarSource {
    pub(crate) fn new(base_dir: &Path, source: &CalendarSourceConfig) -> Result<CalendarSource> {
        log::debug!("creating calendar source: {}", source);
        if let Ok(url) = Url::parse(source.into()) {
            log::debug!("calendar source is a url");
            return Ok(CalendarSource::CalendarUrl(url));
        };

        let path = base_dir
            .join(PathBuf::try_from(source).wrap_err("calendar source is not a valid file path")?);

        if path.exists() {
            log::debug!("calendar source is a file that exists");
            Ok(CalendarSource::CalendarFile(path))
        } else {
            bail!("could not create CalendarSource from: {}", source);
        }
    }

    /// Returns the parsed calendars of this [`CalendarSource`].
    ///
    /// Listed as plural because a single source may contain multiple calendars as per the ical/ics standard.
    pub(crate) fn parse_calendars(
        &self,
        base_dir: &Path,
    ) -> Result<(Vec<Calendar>, HashSet<String>)> {
        let (parsed_calendars, calendar_unparsed_properties) = match self {
            Self::CalendarFile(file) => {
                log::info!("reading calendar file: {:?}", file);
                let buf = BufReader::new(File::open(base_dir.join(file))?);
                Calendar::parse_calendars(buf)?
            }
            Self::CalendarUrl(url) => {
                log::info!("reading calendar url: {}", url);
                let ics_string = ureq::get(url.as_ref()).call()?.into_string()?;
                Calendar::parse_calendars(ics_string.as_bytes())?
            }
        };

        Ok((parsed_calendars, calendar_unparsed_properties))
    }
}
