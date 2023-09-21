use chrono::Duration;
use color_eyre::eyre::{bail, eyre, Context, Result};
use log::debug;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use std::{
    fs::{self, create_dir_all, File},
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    rc::Rc,
};
use url::Url;

use crate::{
    configuration::{
        calendar_source_config::CalendarSourceConfig, config::Config, types::cache_mode::CacheMode,
    },
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
                let ics_string = retrieve_cached_url(config, source_config, url)?;
                Calendar::parse_calendars(ics_string.as_bytes(), source_config.clone())?
            }
        };

        Ok(parsed_calendars)
    }
}

fn retrieve_cached_url(
    config: &Config,
    source_config: &Rc<CalendarSourceConfig>,
    url: &Url,
) -> Result<String, color_eyre::eyre::Error> {
    // setup the cache directory
    // TODO: might want to do this once in the Config or CalendarCollection
    let cache_dir = &config.base_dir.join(&config.cache_dir);
    // create the cache file path
    let mut calendar_cache_file = cache_dir.join(&source_config.name);
    calendar_cache_file.set_extension("ics");

    if config.cache_mode != CacheMode::NeverCache {
        // make the cache directory if it does not exist
        if !cache_dir.exists() {
            create_dir_all(cache_dir).wrap_err("could not create cache dir")?;
        }

        debug!(
            "checking to see if the cache file exists: {:?}",
            calendar_cache_file
        );
        if calendar_cache_file.exists() {
            // get the last modified time
            let cache_file_age = Duration::from_std(
                fs::metadata(&calendar_cache_file)
                    .wrap_err("could not get file metadata for cache file")?
                    .modified()
                    .wrap_err("could not get the last modified time of cache file")?
                    .elapsed()
                    .wrap_err("could not get elapsed time since the file modified date")?,
            )
            .wrap_err("could not convert system duration into Chrono::Duration")?;

            let cache_timeout = config
                .cache_timeout_duration
                .get()
                .ok_or(eyre!("could not get cache_timeout_duration"))?;
            debug!(
                "checking if the cache file is still valid: {} <= {}",
                cache_file_age, cache_timeout
            );
            if cache_file_age <= *cache_timeout {
                let mut file_buffer = String::new();
                File::open(calendar_cache_file)
                    .wrap_err("could not open cache file for read")?
                    .read_to_string(&mut file_buffer)
                    .wrap_err("could not read contents of cache file")?;

                // return the cached calendar contents
                debug!("cache file is valid, returning cached data");
                return Ok(file_buffer);
            }
        }
    }
    // if we did not find a valid cache file, we need to download the data, cache it, and then return it

    if config.cache_mode != CacheMode::NeverDownload {
        // add any provided cookies to the request
        // TODO: we might want to support arbitrary headers later
        let mut headers = HeaderMap::new();
        if let Some(cookies) = &source_config.cookies {
            debug!("Found {} cookies to add to request", cookies.len());
            for cookie in cookies {
                headers.insert(
                    COOKIE,
                    HeaderValue::from_str(cookie)
                        .wrap_err("could not convert provided cookie into valid HeaderValue")?,
                );
            }
        }

        // retrieve the calendar
        debug!("downloading the calendar from: {}", url);
        let response = reqwest::blocking::Client::new()
            .get(url.as_ref())
            .headers(headers)
            .send()
            .wrap_err("could not get content from downloaded calendar")?;

        // throw an error if we are not using the cache and we could not actually download a calendar
        if config.cache_mode == CacheMode::NeverCache || !(response.status()).is_success() {
            let status = &response.status();
            return Err(eyre!(
                "could not download calendar: {}, {:?}",
                status.as_str(),
                status.canonical_reason()
            ));
        }

        // get the response body
        let ics_string = &response
            .text()
            .wrap_err("could not convert calendar to string")?;

        // create the cache file and write the calendar to the cache file
        if config.cache_mode != CacheMode::NeverCache {
            debug!(
                "creating the calendar cache file: {:?}",
                calendar_cache_file
            );
            File::create(calendar_cache_file)
                .wrap_err("could not create the cache file")?
                .write_all(ics_string.as_bytes())
                .wrap_err("could not write the calendar to its cache file")?;
        }

        // return the response body
        return Ok(ics_string.clone());
    }

    Err(eyre!(
        "could not retrieve a cached file or download from the network with the current cache mode"
    ))
}
