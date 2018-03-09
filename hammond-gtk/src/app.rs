#![allow(new_without_default)]

use gio::{ApplicationExt, ApplicationExtManual, ApplicationFlags};
use glib;
use gtk;
use gtk::prelude::*;

use hammond_data::{Podcast, Source};
use hammond_data::utils::checkup;

use appnotif::*;
use headerbar::Header;
use stacks::Content;
use utils;
use widgets::mark_all_watched;

use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

#[derive(Clone, Debug)]
pub enum Action {
    UpdateSources(Option<Source>),
    RefreshAllViews,
    RefreshEpisodesView,
    RefreshEpisodesViewBGR,
    RefreshShowsView,
    RefreshWidget,
    RefreshWidgetIfVis,
    ReplaceWidget(Arc<Podcast>),
    RefreshWidgetIfSame(i32),
    ShowWidgetAnimated,
    ShowShowsAnimated,
    HeaderBarShowTile(String),
    HeaderBarNormal,
    HeaderBarShowUpdateIndicator,
    HeaderBarHideUpdateIndicator,
    MarkAllPlayerNotification(Arc<Podcast>),
}

#[derive(Debug)]
pub struct App {
    app_instance: gtk::Application,
    window: gtk::Window,
    overlay: gtk::Overlay,
    header: Arc<Header>,
    content: Arc<Content>,
    receiver: Receiver<Action>,
    sender: Sender<Action>,
}

impl App {
    pub fn new() -> App {
        let application = gtk::Application::new("org.gnome.Hammond", ApplicationFlags::empty())
            .expect("Application Initialization failed...");

        // Weird magic I copy-pasted that sets the Application Name in the Shell.
        glib::set_application_name("Hammond");
        glib::set_prgname(Some("Hammond"));

        // Create the main window
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_default_size(860, 640);
        window.set_title("Hammond");
        let app_clone = application.clone();
        window.connect_delete_event(move |_, _| {
            app_clone.quit();
            Inhibit(false)
        });

        let (sender, receiver) = channel();

        // Create a content instance
        let content =
            Arc::new(Content::new(sender.clone()).expect("Content Initialization failed."));

        // Create the headerbar
        let header = Arc::new(Header::new(&content, &window, sender.clone()));

        // Add the content main stack to the overlay.
        let overlay = gtk::Overlay::new();
        overlay.add(&content.get_stack());

        // Add the overlay to the main window
        window.add(&overlay);

        App {
            app_instance: application,
            window,
            overlay,
            header,
            content,
            receiver,
            sender,
        }
    }

    fn setup_timed_callbacks(&self) {
        let sender = self.sender.clone();
        // Update the feeds right after the Application is initialized.
        gtk::timeout_add_seconds(2, move || {
            utils::refresh_feed_wrapper(None, sender.clone());
            glib::Continue(false)
        });

        let sender = self.sender.clone();
        // Auto-updater, runs every hour.
        // TODO: expose the interval in which it run to a user setting.
        gtk::timeout_add_seconds(3600, move || {
            utils::refresh_feed_wrapper(None, sender.clone());
            glib::Continue(true)
        });

        // Run a database checkup once the application is initialized.
        gtk::timeout_add(300, || {
            if let Err(err) = checkup() {
                error!("Check up failed: {}", err);
            }

            glib::Continue(false)
        });
    }

    pub fn run(self) {
        let window = self.window.clone();
        self.app_instance.connect_startup(move |app| {
            build_ui(&window, app);
        });
        self.setup_timed_callbacks();

        let content = self.content.clone();
        let headerbar = self.header.clone();
        let sender = self.sender.clone();
        let overlay = self.overlay.clone();
        let receiver = self.receiver;
        gtk::idle_add(move || {
            match receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(Action::UpdateSources(source)) => {
                    if let Some(s) = source {
                        utils::refresh_feed_wrapper(Some(vec![s]), sender.clone());
                    } else {
                        utils::refresh_feed_wrapper(None, sender.clone());
                    }
                }
                Ok(Action::RefreshAllViews) => content.update(),
                Ok(Action::RefreshShowsView) => content.update_shows_view(),
                Ok(Action::RefreshWidget) => content.update_widget(),
                Ok(Action::RefreshWidgetIfVis) => content.update_widget_if_visible(),
                Ok(Action::RefreshWidgetIfSame(id)) => content.update_widget_if_same(id),
                Ok(Action::RefreshEpisodesView) => content.update_episode_view(),
                Ok(Action::RefreshEpisodesViewBGR) => content.update_episode_view_if_baground(),
                Ok(Action::ReplaceWidget(pd)) => {
                    if let Err(err) = content.get_shows().replace_widget(pd) {
                        error!("Something went wrong while trying to update the ShowWidget.");
                        error!("Error: {}", err);
                    }
                }
                Ok(Action::ShowWidgetAnimated) => content.get_shows().switch_widget_animated(),
                Ok(Action::ShowShowsAnimated) => content.get_shows().switch_podcasts_animated(),
                Ok(Action::HeaderBarShowTile(title)) => headerbar.switch_to_back(&title),
                Ok(Action::HeaderBarNormal) => headerbar.switch_to_normal(),
                Ok(Action::HeaderBarShowUpdateIndicator) => headerbar.show_update_notification(),
                Ok(Action::HeaderBarHideUpdateIndicator) => headerbar.hide_update_notification(),
                Ok(Action::MarkAllPlayerNotification(pd)) => {
                    let sender = sender.clone();
                    let callback = clone!(sender => move || {
                        if let Err(err) = mark_all_watched(&pd, sender.clone()) {
                            error!("Something went horribly wrong with the notif callback: {}", err);
                        }
                        glib::Continue(false)
                    });
                    let text = "All episodes where marked as watched.";

                    let sender = sender.clone();
                    let notif = InAppNotification::new(text.into(), callback, sender);
                    overlay.add_overlay(&notif.overlay);
                    overlay.show_all();
                }
                Err(_) => (),
            }

            Continue(true)
        });

        ApplicationExtManual::run(&self.app_instance, &[]);
    }
}

fn build_ui(window: &gtk::Window, app: &gtk::Application) {
    window.set_application(app);
    window.show_all();
    window.activate();
    app.connect_activate(move |_| ());
}
