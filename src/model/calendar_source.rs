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
        if let Ok(url) = Url::parse(source) {
            return Ok(CalendarSource::CalendarUrl(url));
        };
        let path = PathBuf::try_from(source)?;
        if path.exists() {
            Ok(CalendarSource::CalendarFile(path))
        } else {
            bail!("could not create CalendarSource from: {}", source);
        }
    }

    pub(crate) fn from_strings(sources: Vec<String>) -> Vec<Result<CalendarSource>> {
        sources
            .iter()
            .map(|cal_str| CalendarSource::new(cal_str))
            .collect()
    }
}
