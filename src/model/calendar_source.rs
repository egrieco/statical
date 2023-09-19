use color_eyre::eyre::{bail, eyre, Context, Result};
use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    rc::Rc,
};
use url::Url;

use crate::{
    configuration::{calendar_source_config::CalendarSourceConfig, config::Config},
    model::calendar::Calendar,
};

#[derive(Debug)]
pub(crate) enum CalendarSource {
    CalendarUrl(Url, Rc<CalendarSourceConfig>),
    CalendarFile(PathBuf, Rc<CalendarSourceConfig>),
}

impl CalendarSource {
    pub(crate) fn new(
        base_dir: &Path,
        source_config: Rc<CalendarSourceConfig>,
        config: &Config,
    ) -> Result<CalendarSource> {
        // adjust the color here if the config instructs us to
        source_config
            .adjusted_color
            .set(source_config.color.adjust_color(config))
            .map_err(|e| eyre!(e))
            .wrap_err("could not adjust color")?;

        log::debug!("creating calendar source: {}", source_config);
        if let Ok(url) = Url::parse(&source_config.source) {
            log::debug!("calendar source is a url");
            return Ok(CalendarSource::CalendarUrl(url, source_config));
        };

        let path = base_dir.join(
            PathBuf::try_from(&source_config.source)
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
    pub(crate) fn parse_calendars(&self, config: &Config) -> Result<Vec<Calendar>> {
        let base_dir: &Path = &config.base_dir;
        let parsed_calendars = match self {
            Self::CalendarFile(file, source_config) => {
                log::info!("reading calendar file: {:?}", file);
                let buf = BufReader::new(File::open(base_dir.join(file))?);
                Calendar::parse_calendars(buf, source_config.clone())?
            }
            Self::CalendarUrl(url, source_config) => {
                log::info!("reading calendar url: {}", url);
                let ics_string = ureq::get(url.as_ref()).call()?.into_string()?;
                Calendar::parse_calendars(ics_string.as_bytes(), source_config.clone())?
            }
        };

        Ok(parsed_calendars)
    }
}
