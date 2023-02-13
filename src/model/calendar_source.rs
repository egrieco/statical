use color_eyre::eyre::{bail, Result};
use std::path::PathBuf;
use url::Url;

#[derive(Debug)]
pub(crate) enum CalendarSource {
    CalendarUrl(Url),
    CalendarFile(PathBuf),
}

impl CalendarSource {
    pub(crate) fn new(source: &str) -> Result<CalendarSource> {
        log::debug!("creating calendar source: {}", source);
        if let Ok(url) = Url::parse(source) {
            log::debug!("calendar source is a url");
            return Ok(CalendarSource::CalendarUrl(url));
        };
        let path = PathBuf::try_from(source)?;
        if path.exists() {
            log::debug!("calendar source is a file that exists");
            Ok(CalendarSource::CalendarFile(path))
        } else {
            bail!("could not create CalendarSource from: {}", source);
        }
    }

    /// Create calendar sources from strings
    ///
    /// Fail immediately if any of the sources is invalid
    pub(crate) fn from_strings(cal_strs: Vec<String>) -> Result<Vec<CalendarSource>> {
        let mut sources = vec![];
        for cal_str in cal_strs {
            sources.push(CalendarSource::new(&cal_str)?);
        }
        Ok(sources)
    }
}
