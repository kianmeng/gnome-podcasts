use glib;
use gtk;

use chrono::prelude::*;
use gtk::prelude::*;

use chrono::Duration;
use failure::Error;
use humansize::{file_size_opts as size_opts, FileSize};
use open;

use hammond_data::{EpisodeWidgetQuery, Podcast};
use hammond_data::dbqueries;
use hammond_data::utils::get_download_folder;

use app::Action;
use manager;
use widgets::episode_states::*;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

lazy_static! {
    static ref SIZE_OPTS: Arc<size_opts::FileSizeOpts> =  {
        // Declare a custom humansize option struct
        // See: https://docs.rs/humansize/1.0.2/humansize/file_size_opts/struct.FileSizeOpts.html
        Arc::new(size_opts::FileSizeOpts {
            divider: size_opts::Kilo::Binary,
            units: size_opts::Kilo::Decimal,
            decimal_places: 0,
            decimal_zeroes: 0,
            fixed_at: size_opts::FixedAt::No,
            long_units: false,
            space: true,
            suffix: "",
            allow_negative: false,
        })
    };

    static ref NOW: DateTime<Utc> = Utc::now();
}

#[derive(Debug, Clone)]
pub struct EpisodeWidget {
    pub container: gtk::Box,
    play: gtk::Button,
    download: gtk::Button,
    cancel: gtk::Button,
    title: TitleMachine,
    date: gtk::Label,
    duration: gtk::Label,
    progress: gtk::ProgressBar,
    total_size: gtk::Label,
    local_size: gtk::Label,
    separator1: gtk::Label,
    separator2: gtk::Label,
    prog_separator: gtk::Label,
}

impl Default for EpisodeWidget {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/episode_widget.ui");

        let container: gtk::Box = builder.get_object("episode_container").unwrap();
        let progress: gtk::ProgressBar = builder.get_object("progress_bar").unwrap();

        let download: gtk::Button = builder.get_object("download_button").unwrap();
        let play: gtk::Button = builder.get_object("play_button").unwrap();
        let cancel: gtk::Button = builder.get_object("cancel_button").unwrap();

        let title: gtk::Label = builder.get_object("title_label").unwrap();
        let date: gtk::Label = builder.get_object("date_label").unwrap();
        let duration: gtk::Label = builder.get_object("duration_label").unwrap();
        let local_size: gtk::Label = builder.get_object("local_size").unwrap();
        let total_size: gtk::Label = builder.get_object("total_size").unwrap();

        let separator1: gtk::Label = builder.get_object("separator1").unwrap();
        let separator2: gtk::Label = builder.get_object("separator2").unwrap();
        let prog_separator: gtk::Label = builder.get_object("prog_separator").unwrap();

        let title_machine = TitleMachine::new(title, false);

        EpisodeWidget {
            container,
            progress,
            download,
            play,
            cancel,
            title: title_machine,
            duration,
            date,
            total_size,
            local_size,
            separator1,
            separator2,
            prog_separator,
        }
    }
}

impl EpisodeWidget {
    pub fn new(episode: EpisodeWidgetQuery, sender: Sender<Action>) -> EpisodeWidget {
        let widget = EpisodeWidget::default();
        widget.init(episode, sender)
    }

    fn init(mut self, episode: EpisodeWidgetQuery, sender: Sender<Action>) -> Self {
        WidgetExt::set_name(&self.container, &episode.rowid().to_string());

        // Set the title label state.
        self = self.set_title(&episode);

        // Set the duaration label.
        self.set_duration(episode.duration());

        // Set the date label.
        self.set_date(episode.epoch());

        // Show or hide the play/delete/download buttons upon widget initialization.
        if let Err(err) = self.show_buttons(episode.local_uri()) {
            error!("Failed to determine play/download button state.");
            error!("Error: {}", err);
        }

        // Set the size label.
        if let Err(err) = self.set_total_size(episode.length()) {
            error!("Failed to set the Size label.");
            error!("Error: {}", err);
        }

        // Determine what the state of the progress bar should be.
        if let Err(err) = self.determine_progess_bar() {
            error!("Something went wrong determining the ProgressBar State.");
            error!("Error: {}", err);
        }

        let episode = Arc::new(Mutex::new(episode));

        self.play
            .connect_clicked(clone!(episode, sender => move |_| {
                if let Ok(mut ep) = episode.lock() {
                    if let Err(err) = on_play_bttn_clicked(&mut ep, sender.clone()){
                        error!("Error: {}", err);
                    };
                }
        }));

        self.download
            .connect_clicked(clone!(episode, sender => move |dl| {
                dl.set_sensitive(false);
                if let Ok(ep) = episode.lock() {
                    if let Err(err) = on_download_clicked(&ep, sender.clone())  {
                        error!("Download failed to start.");
                        error!("Error: {}", err);
                    } else {
                        info!("Donwload started succesfully.");
                    }
                }
        }));

        self
    }

    /// Show or hide the play/delete/download buttons upon widget initialization.
    fn show_buttons(&self, local_uri: Option<&str>) -> Result<(), Error> {
        let path = local_uri.ok_or_else(|| format_err!("Path is None"))?;
        if Path::new(path).exists() {
            self.download.hide();
            self.play.show();
        }
        Ok(())
    }

    /// Determine the title state.
    fn set_title(mut self, episode: &EpisodeWidgetQuery) -> Self {
        self.title.set_title(episode.title());
        self.title = self.title.determine_state(episode.played().is_some());
        self
    }

    /// Set the date label depending on the current time.
    fn set_date(&self, epoch: i32) {
        let date = Utc.timestamp(i64::from(epoch), 0);
        if NOW.year() == date.year() {
            self.date.set_text(date.format("%e %b").to_string().trim());
        } else {
            self.date
                .set_text(date.format("%e %b %Y").to_string().trim());
        };
    }

    /// Set the duration label.
    fn set_duration(&self, seconds: Option<i32>) -> Option<()> {
        let minutes = Duration::seconds(seconds?.into()).num_minutes();
        if minutes == 0 {
            return None;
        }

        self.duration.set_text(&format!("{} min", minutes));
        self.duration.show();
        self.separator1.show();
        Some(())
    }

    /// Set the Episode label dependings on its size
    fn set_total_size(&self, bytes: Option<i32>) -> Result<(), Error> {
        let size = bytes.ok_or_else(|| format_err!("Size is None."))?;
        if size == 0 {
            bail!("Size is 0.");
        }

        let s = size.file_size(SIZE_OPTS.clone())
            .map_err(|err| format_err!("{}", err))?;
        self.total_size.set_text(&s);
        self.total_size.show();
        self.separator2.show();
        Ok(())
    }

    // FIXME: REFACTOR ME
    // Something Something State-Machine?
    fn determine_progess_bar(&self) -> Result<(), Error> {
        let id = WidgetExt::get_name(&self.container)
            .ok_or_else(|| format_err!("Failed to get widget Name"))?
            .parse::<i32>()?;

        let active_dl = || -> Result<Option<_>, Error> {
            let m = manager::ACTIVE_DOWNLOADS
                .read()
                .map_err(|_| format_err!("Failed to get a lock on the mutex."))?;

            Ok(m.get(&id).cloned())
        }()?;

        if let Some(prog) = active_dl {
            // FIXME: Document me?
            self.download.hide();
            self.progress.show();
            self.local_size.show();
            self.total_size.show();
            self.separator2.show();
            self.prog_separator.show();
            self.cancel.show();

            let progress_bar = self.progress.clone();
            let total_size = self.total_size.clone();
            let local_size = self.local_size.clone();

            // Setup a callback that will update the progress bar.
            update_progressbar_callback(prog.clone(), id, &progress_bar, &local_size);

            // Setup a callback that will update the total_size label
            // with the http ContentLength header number rather than
            // relying to the RSS feed.
            update_total_size_callback(prog.clone(), &total_size);

            self.cancel.connect_clicked(clone!(prog => move |cancel| {
                if let Ok(mut m) = prog.lock() {
                    m.cancel();
                    cancel.set_sensitive(false);
                }
            }));
        }

        Ok(())
    }
}

fn on_download_clicked(ep: &EpisodeWidgetQuery, sender: Sender<Action>) -> Result<(), Error> {
    let pd = dbqueries::get_podcast_from_id(ep.podcast_id())?;
    let download_fold = get_download_folder(&pd.title().to_owned())?;

    // Start a new download.
    manager::add(ep.rowid(), &download_fold, sender.clone())?;

    // Update Views
    sender.send(Action::RefreshEpisodesView)?;
    sender.send(Action::RefreshWidgetIfVis)?;

    Ok(())
}

fn on_play_bttn_clicked(
    episode: &mut EpisodeWidgetQuery,
    sender: Sender<Action>,
) -> Result<(), Error> {
    open_uri(episode.rowid())?;
    episode.set_played_now()?;

    sender.send(Action::RefreshWidgetIfVis)?;
    sender.send(Action::RefreshEpisodesView)?;
    Ok(())
}

fn open_uri(rowid: i32) -> Result<(), Error> {
    let uri = dbqueries::get_episode_local_uri_from_id(rowid)?
        .ok_or_else(|| format_err!("Expected Some found None."))?;

    if Path::new(&uri).exists() {
        info!("Opening {}", uri);
        open::that(&uri)?;
    } else {
        bail!("File \"{}\" does not exist.", uri);
    }

    Ok(())
}

// Setup a callback that will update the progress bar.
#[cfg_attr(feature = "cargo-clippy", allow(if_same_then_else))]
fn update_progressbar_callback(
    prog: Arc<Mutex<manager::Progress>>,
    episode_rowid: i32,
    progress_bar: &gtk::ProgressBar,
    local_size: &gtk::Label,
) {
    timeout_add(
        400,
        clone!(prog, progress_bar, progress_bar, local_size=> move || {
            progress_bar_helper(prog.clone(), episode_rowid, &progress_bar, &local_size)
                .unwrap_or(glib::Continue(false))
        }),
    );
}

fn progress_bar_helper(
    prog: Arc<Mutex<manager::Progress>>,
    episode_rowid: i32,
    progress_bar: &gtk::ProgressBar,
    local_size: &gtk::Label,
) -> Result<glib::Continue, Error> {
    let (fraction, downloaded) = {
        let m = prog.lock()
            .map_err(|_| format_err!("Failed to get a lock on the mutex."))?;
        (m.get_fraction(), m.get_downloaded())
    };

    // Update local_size label
    downloaded
        .file_size(SIZE_OPTS.clone())
        .map_err(|err| format_err!("{}", err))
        .map(|x| local_size.set_text(&x))?;

    // I hate floating points.
    // Update the progress_bar.
    if (fraction >= 0.0) && (fraction <= 1.0) && (!fraction.is_nan()) {
        progress_bar.set_fraction(fraction);
    }

    // info!("Fraction: {}", progress_bar.get_fraction());
    // info!("Fraction: {}", fraction);

    // Check if the download is still active
    let active = {
        let m = manager::ACTIVE_DOWNLOADS
            .read()
            .map_err(|_| format_err!("Failed to get a lock on the mutex."))?;
        m.contains_key(&episode_rowid)
    };

    if (fraction >= 1.0) && (!fraction.is_nan()) {
        Ok(glib::Continue(false))
    } else if !active {
        Ok(glib::Continue(false))
    } else {
        Ok(glib::Continue(true))
    }
}

// Setup a callback that will update the total_size label
// with the http ContentLength header number rather than
// relying to the RSS feed.
fn update_total_size_callback(prog: Arc<Mutex<manager::Progress>>, total_size: &gtk::Label) {
    timeout_add(
        500,
        clone!(prog, total_size => move || {
            total_size_helper(prog.clone(), &total_size).unwrap_or(glib::Continue(true))
        }),
    );
}

fn total_size_helper(
    prog: Arc<Mutex<manager::Progress>>,
    total_size: &gtk::Label,
) -> Result<glib::Continue, Error> {
    // Get the total_bytes.
    let total_bytes = {
        let m = prog.lock()
            .map_err(|_| format_err!("Failed to get a lock on the mutex."))?;
        m.get_total_size()
    };

    debug!("Total Size: {}", total_bytes);
    if total_bytes != 0 {
        // Update the total_size label
        total_bytes
            .file_size(SIZE_OPTS.clone())
            .map_err(|err| format_err!("{}", err))
            .map(|x| total_size.set_text(&x))?;
        // Do not call again the callback
        Ok(glib::Continue(false))
    } else {
        Ok(glib::Continue(true))
    }
}

// fn on_delete_bttn_clicked(episode_id: i32) -> Result<(), Error> {
//     let mut ep = dbqueries::get_episode_from_rowid(episode_id)?.into();
//     delete_local_content(&mut ep).map_err(From::from).map(|_| ())
// }

pub fn episodes_listbox(pd: &Podcast, sender: Sender<Action>) -> Result<gtk::ListBox, Error> {
    let episodes = dbqueries::get_pd_episodeswidgets(pd)?;

    let list = gtk::ListBox::new();

    episodes.into_iter().for_each(|ep| {
        let widget = EpisodeWidget::new(ep, sender.clone());
        list.add(&widget.container);
    });

    list.set_vexpand(false);
    list.set_hexpand(false);
    list.set_visible(true);
    list.set_selection_mode(gtk::SelectionMode::None);
    Ok(list)
}
