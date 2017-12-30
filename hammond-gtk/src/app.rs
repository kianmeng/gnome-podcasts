use gtk;
use glib;
use gio;
use gtk::prelude::*;
use gio::{ActionMapExt, ApplicationExt, ApplicationExtManual, SimpleActionExt};

use hammond_data::utils::checkup;

use headerbar::Header;
use content::Content;
use utils;

use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct App {
    app_instance: gtk::Application,
    window: gtk::Window,
    header: Rc<Header>,
    content: Rc<Content>,
}

impl App {
    pub fn new() -> App {
        let application =
            gtk::Application::new("org.gnome.Hammond", gio::ApplicationFlags::empty())
                .expect("Initialization failed...");

        // Weird magic I copy-pasted that sets the Application Name in the Shell.
        glib::set_application_name("Hammond");
        glib::set_prgname(Some("Hammond"));

        // Create the main window
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_default_size(860, 640);
        window.set_title("Hammond");
        window.connect_delete_event(|w, _| {
            w.destroy();
            Inhibit(false)
        });

        // TODO: Refactor the initialization order.

        // Create the headerbar
        let header = Rc::new(Header::default());

        // Create a content instance
        let content = Content::new(header.clone());

        // Initialize the headerbar
        header.init(content.clone());

        // Add the Headerbar to the window.
        window.set_titlebar(&header.container);
        // Add the content main stack to the window.
        window.add(&content.get_stack());

        App {
            app_instance: application,
            window,
            header,
            content,
        }
    }

    pub fn setup_actions(&self) {
        // Updates the database and refreshes every view.
        let update = gio::SimpleAction::new("update", None);
        let content = self.content.clone();
        let header = self.header.clone();
        update.connect_activate(move |_, _| {
            utils::refresh_feed(content.clone(), header.clone(), None);
        });
        self.app_instance.add_action(&update);

        // Refreshes the `Content`
        let refresh = gio::SimpleAction::new("refresh", None);
        let content = self.content.clone();
        refresh.connect_activate(move |_, _| {
            content.update();
        });
        self.app_instance.add_action(&refresh);

        // Refreshes the `EpisodesStack`
        let refresh_episodes = gio::SimpleAction::new("refresh_episodes", None);
        let content = self.content.clone();
        refresh_episodes.connect_activate(move |_, _| {
            if content.get_stack().get_visible_child_name() != Some(String::from("episodes")) {
                content.update_episode_view();
            }
        });
        self.app_instance.add_action(&refresh_episodes);

        // Refreshes the `ShowStack`
        let refresh_shows = gio::SimpleAction::new("refresh_shows", None);
        let content = self.content.clone();
        refresh_shows.connect_activate(move |_, _| {
            content.update_shows_view();
        });
        self.app_instance.add_action(&refresh_shows);
    }

    pub fn setup_timed_callbacks(&self) {
        let content = self.content.clone();
        let header = self.header.clone();
        // Update 30 seconds after the Application is initialized.
        gtk::timeout_add_seconds(
            30,
            clone!(content => move || {
            utils::refresh_feed(content.clone(), header.clone(), None);
            glib::Continue(false)
        }),
        );

        let content = self.content.clone();
        let header = self.header.clone();
        // Auto-updater, runs every hour.
        // TODO: expose the interval in which it run to a user setting.
        // TODO: show notifications.
        gtk::timeout_add_seconds(
            3600,
            clone!(content => move || {
            utils::refresh_feed(content.clone(), header.clone(), None);
            glib::Continue(true)
        }),
        );

        // Run a database checkup once the application is initialized.
        gtk::idle_add(move || {
            let _ = checkup();
            glib::Continue(false)
        });
    }

    pub fn run(&self) {
        let window = self.window.clone();
        let app = self.app_instance.clone();
        self.app_instance.connect_startup(move |_| {
            build_ui(&window, &app);
        });
        self.setup_timed_callbacks();
        self.setup_actions();

        ApplicationExtManual::run(&self.app_instance, &[]);
    }
}

fn build_ui(window: &gtk::Window, app: &gtk::Application) {
    window.set_application(app);
    window.show_all();
    window.activate();
    app.connect_activate(move |_| ());
}