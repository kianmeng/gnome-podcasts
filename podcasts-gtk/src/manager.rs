// manager.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;

use podcasts_data::dbqueries;
use podcasts_data::downloader::{get_episode, DownloadProgress};

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

// This is messy, undocumented and hacky af.
// I am terrible at writing downloaders and download managers.

pub(crate) static ACTIVE_DOWNLOADS: Lazy<Arc<RwLock<HashMap<i32, Arc<Mutex<Progress>>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
static DLPOOL: Lazy<rayon::ThreadPool> =
    Lazy::new(|| rayon::ThreadPoolBuilder::new().build().unwrap());

#[derive(Debug)]
pub(crate) struct Progress {
    total_bytes: u64,
    downloaded_bytes: u64,
    cancel: bool,
}

impl Default for Progress {
    fn default() -> Self {
        Progress {
            total_bytes: 0,
            downloaded_bytes: 0,
            cancel: false,
        }
    }
}

impl Progress {
    pub(crate) fn get_fraction(&self) -> f64 {
        let ratio = self.downloaded_bytes as f64 / self.total_bytes as f64;
        debug!("{:?}", self);
        debug!("Ratio completed: {}", ratio);

        if ratio >= 1.0 {
            return 1.0;
        };
        ratio
    }
}

impl DownloadProgress for Progress {
    fn get_downloaded(&self) -> u64 {
        self.downloaded_bytes
    }

    fn set_downloaded(&mut self, downloaded: u64) {
        self.downloaded_bytes = downloaded
    }

    fn set_size(&mut self, bytes: u64) {
        self.total_bytes = bytes;
    }

    fn get_size(&self) -> u64 {
        self.total_bytes
    }

    fn should_cancel(&self) -> bool {
        self.cancel
    }

    fn cancel(&mut self) {
        self.cancel = true;
    }
}

pub(crate) fn add(id: i32, directory: String) -> Result<()> {
    // Create a new `Progress` struct to keep track of dl progress.
    let prog = Arc::new(Mutex::new(Progress::default()));

    match ACTIVE_DOWNLOADS.write() {
        Ok(mut guard) => guard.insert(id, prog.clone()),
        Err(err) => return Err(anyhow!("ActiveDownloads: {}.", err)),
    };

    DLPOOL.spawn(move || {
        if let Ok(mut episode) = dbqueries::get_episode_widget_from_rowid(id) {
            let id = episode.rowid();

            get_episode(&mut episode, directory.as_str(), Some(prog))
                .map_err(|err| error!("Download Failed: {}", err))
                .ok();

            if let Ok(mut m) = ACTIVE_DOWNLOADS.write() {
                let progress = m.remove(&id);
                debug!("Removed: {:?}", progress);
            }

            // if let Ok(m) = ACTIVE_DOWNLOADS.read() {
            //     debug!("ACTIVE DOWNLOADS: {:#?}", m);
            // }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use podcasts_data::dbqueries;
    use podcasts_data::pipeline::pipeline;
    use podcasts_data::utils::get_download_folder;
    use podcasts_data::{Episode, Save, Source};

    use podcasts_data::downloader::get_episode;

    use std::fs;
    use std::path::Path;
    use std::{thread, time};

    #[test]
    // This test inserts an rss feed to your `XDG_DATA/podcasts/podcasts.db` so we make it explicit
    // to run it.
    #[ignore]
    // THIS IS NOT A RELIABLE TEST
    // Just quick sanity check
    fn test_start_dl() -> Result<()> {
        let url = "https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff";
        // Create and index a source
        let mut source = Source::from_url(url)?;
        // Copy its id
        let sid = source.id();
        source.set_http_etag(None);
        source.set_last_modified(None);
        source.save()?;
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(pipeline(vec![source]));

        // Get the podcast
        let pd = dbqueries::get_podcast_from_source_id(sid)?;
        let title = "Coming soon... The Tip Off";
        // Get an episode
        let episode: Episode = dbqueries::get_episode_from_pk(title, pd.id())?;

        let download_fold = get_download_folder(&pd.title())?;
        let fold2 = download_fold.clone();
        add(episode.rowid(), download_fold)?;
        assert_eq!(ACTIVE_DOWNLOADS.read().unwrap().len(), 1);

        // Give it some time to download the file
        thread::sleep(time::Duration::from_secs(20));

        let final_path = format!("{}/{}.mp3", &fold2, episode.rowid());
        assert_eq!(ACTIVE_DOWNLOADS.read().unwrap().len(), 0);
        assert!(Path::new(&final_path).exists());
        fs::remove_file(final_path)?;
        Ok(())
    }

    #[test]
    // This test needs access to local system so we ignore it by default.
    #[ignore]
    fn test_dl_steal_the_stars() -> Result<()> {
        let url =
            "https://web.archive.org/web/20180120104957if_/https://rss.art19.com/steal-the-stars";
        // Create and index a source
        let mut source = Source::from_url(url)?;
        // Copy its id
        let sid = source.id();
        source.set_http_etag(None);
        source.set_last_modified(None);
        source.save()?;
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(pipeline(vec![source]));

        // Get the podcast
        let pd = dbqueries::get_podcast_from_source_id(sid)?;
        let title = "Introducing Steal the Stars";
        // Get an episode
        let mut episode = dbqueries::get_episode_from_pk(title, pd.id())?.into();
        let download_fold = get_download_folder(&pd.title())?;

        get_episode(&mut episode, &download_fold, None)?;

        let final_path = format!("{}/{}.mp3", &download_fold, episode.rowid());
        assert!(Path::new(&final_path).exists());
        fs::remove_file(final_path)?;
        Ok(())
    }
}
